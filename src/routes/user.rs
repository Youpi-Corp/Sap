use crate::application::services::UserService;
use crate::domain::models::{NewUserObject, UserObject};
use crate::infrastructure::persistence::user_repository::PostgresUserRepository;
use actix_web::{web, Error, HttpResponse, Responder};
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;

fn with_user_service<F>(
    pool: &web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    f: F,
) -> Result<HttpResponse, Error>
where
    F: FnOnce(&mut UserService<PostgresUserRepository>) -> Result<HttpResponse, Error>,
{
    let mut conn = pool.get().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to get a connection from the pool: {}",
            e
        ))
    })?;

    let mut user_repo = PostgresUserRepository { conn: &mut conn };
    let mut user_service = UserService::new(&mut user_repo);

    f(&mut user_service)
}

#[utoipa::path(
    get,
    path = "/user/get/{user_id}",
    responses(
        (status = 200, description = "User found successfully", body = UserObject),
        (status = 404, description = "User not found")
    ),
    params(
        ("user_id" = i32, Path, description = "User ID to fetch")
    ),
    tag = "Users"
)]
pub async fn get_user_handler(
    pool: web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    user_id: web::Path<i32>,
) -> impl Responder {
    with_user_service(&pool, |user_service| {
        match user_service.get_user_by_id(*user_id) {
            Ok(user) => Ok(HttpResponse::Ok().json(user)),
            Err(_) => Ok(HttpResponse::NotFound().json("User not found!")),
        }
    })
}

#[utoipa::path(
    post,
    path = "/user/create",
    request_body = NewUserObject,
    responses(
        (status = 200, description = "User created successfully", body = UserObject),
        (status = 500, description = "Failed to create user")
    ),
    tag = "Users"
)]
pub async fn create_user_handler(
    pool: web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    query: web::Json<NewUserObject>,
) -> impl Responder {
    with_user_service(&pool, |user_service| {
        match user_service.create_user(query.into_inner()) {
            Ok(user) => Ok(HttpResponse::Ok().json(user)),
            Err(_) => Ok(HttpResponse::InternalServerError().json("Failed to create user!")),
        }
    })
}

#[utoipa::path(
    get,
    path = "/user/list",
    responses(
        (status = 200, description = "List of users retrieved successfully", body = Vec<UserObject>),
        (status = 500, description = "Failed to retrieve users")
    ),
    tag = "Users"
)]
pub async fn list_users_handler(
    pool: web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
) -> impl Responder {
    with_user_service(&pool, |user_service| match user_service.get_all_users() {
        Ok(users) => Ok(HttpResponse::Ok().json(users)),
        Err(_) => Ok(HttpResponse::InternalServerError().json("Failed to get users!")),
    })
}

#[utoipa::path(
    delete,
    path = "/user/delete/{user_id}",
    responses(
        (status = 200, description = "User deleted successfully"),
        (status = 500, description = "Failed to delete user")
    ),
    params(
        ("user_id" = i32, Path, description = "User ID to delete")
    ),
    tag = "Users"
)]
pub async fn delete_user_handler(
    pool: web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    user_id: web::Path<i32>,
) -> impl Responder {
    with_user_service(&pool, |user_service| {
        match user_service.delete_user(*user_id) {
            Ok(_) => Ok(HttpResponse::Ok().json("User deleted!")),
            Err(_) => Ok(HttpResponse::InternalServerError().json("Failed to delete user!")),
        }
    })
}

#[utoipa::path(
    put,
    path = "/user/update/{user_id}",
    request_body = NewUserObject,
    responses(
        (status = 200, description = "User updated successfully", body = UserObject),
        (status = 500, description = "Failed to update user")
    ),
    params(
        ("user_id" = i32, Path, description = "User ID to update")
    ),
    tag = "Users"
)]
pub async fn update_user_handler(
    pool: web::Data<r2d2::Pool<ConnectionManager<PgConnection>>>,
    user_id: web::Path<i32>,
    query: web::Json<NewUserObject>,
) -> impl Responder {
    with_user_service(&pool, |user_service| {
        let mut user_to_update = user_service.get_user_by_id(*user_id).unwrap();

        // Update the user with the new values if they are provided
        if let Some(pseudo) = &query.pseudo {
            user_to_update.pseudo = Some(pseudo.clone());
        }
        if let Some(email) = &query.email {
            user_to_update.email = Some(email.clone());
        }
        if let Some(password_hash) = &query.password_hash {
            user_to_update.password_hash = Some(password_hash.clone());
        }
        if let Some(role) = &query.role {
            user_to_update.role = Some(role.clone());
        }

        match user_service.update_user(user_to_update) {
            Ok(user) => Ok(HttpResponse::Ok().json(user)),
            Err(_) => Ok(HttpResponse::InternalServerError().json("Failed to update user!")),
        }
    })
}

// Register all user-related routes
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/create", web::post().to(create_user_handler)) // POST /user/create
            .route("/get/{user_id}", web::get().to(get_user_handler)) // GET /user/get
            .route("/list", web::get().to(list_users_handler)) // GET /user/list
            .route("/delete/{user_id}", web::delete().to(delete_user_handler)) // DELETE /user/delete
            .route("/update/{user_id}", web::put().to(update_user_handler)), // PUT /user/update

                                                                             //.route("/login", web::post().to(login_user_handler)) // POST /user/login
                                                                             //.route("/logout", web::post().to(logout_user_handler)) // POST /user/logout
                                                                             //.route("/forgot-password", web::post().to(forgot_password_handler)) // POST /user/forgot-password
                                                                             //.route("/reset-password", web::post().to(reset_password_handler)) // POST /user/reset-password
                                                                             //.route("/report", web::get().to(report_user_handler)) // GET /user/report

                                                                             // sub-scope for role related routes
                                                                             //.service(
                                                                             //    web::scope("/role")
                                                                             //        .route("/get", web::get().to(get_user_role_handler)) // GET /user/role/get
                                                                             //        .route("/set", web::post().to(set_user_role_handler)), // POST /user/role/set
    );
}