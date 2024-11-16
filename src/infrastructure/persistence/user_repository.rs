use crate::domain::{
    models::{Claims, NewUserObject, UserObject},
    repository::UserRepository,
};
use diesel::result::Error;
use diesel::PgConnection;
use diesel::{
    r2d2::{self, ConnectionManager},
    ExpressionMethods, QueryDsl, RunQueryDsl,
};
use dotenvy::dotenv;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::env;

pub struct PostgresUserRepository<'a> {
    pub conn: &'a mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
}

impl<'a> UserRepository for PostgresUserRepository<'a> {
    fn create_user(&mut self, new_user: NewUserObject) -> Result<UserObject, Error> {
        use crate::schema::user::dsl::*;

        // Hash the password
        let hashed_password = bcrypt::hash(
            new_user.password_hash.as_ref().unwrap(),
            bcrypt::DEFAULT_COST,
        )
        .expect("Failed to hash password");

        let new_user = NewUserObject {
            pseudo: new_user.pseudo.clone(),
            email: new_user.email.clone(),
            password_hash: Some(hashed_password),
            role: new_user.role.clone(),
        };

        let result = diesel::insert_into(user)
            .values(&new_user)
            .returning((id, pseudo, email, password_hash, role))
            .get_result::<UserObject>(self.conn);

        match result {
            Ok(user_object) => Ok(user_object),
            Err(e) => Err(e),
        }
    }

    fn get_user_by_id(&mut self, user_id: i32) -> Result<UserObject, Error> {
        use crate::schema::user::dsl::*;

        let result = user
            .filter(id.eq(user_id))
            .select((id, pseudo, email, password_hash, role))
            .first::<UserObject>(self.conn);

        match result {
            Ok(user_object) => Ok(user_object),
            Err(e) => Err(e),
        }
    }

    fn get_all_users(&mut self) -> Result<Vec<UserObject>, Error> {
        use crate::schema::user::dsl::*;

        let result = user
            .select((id, pseudo, email, password_hash, role))
            .load::<UserObject>(self.conn);

        match result {
            Ok(users) => Ok(users),
            Err(e) => Err(e),
        }
    }

    fn delete_user(&mut self, user_id: i32) -> Result<usize, Error> {
        use crate::schema::user::dsl::*;

        let result = diesel::delete(user.filter(id.eq(user_id))).execute(self.conn);

        match result {
            Ok(count) => Ok(count),
            Err(e) => Err(e),
        }
    }

    fn update_user(&mut self, user_object: UserObject) -> Result<UserObject, Error> {
        use crate::schema::user::dsl::*;

        // Hash the password
        let hashed_password = bcrypt::hash(
            user_object.password_hash.as_ref().unwrap(),
            bcrypt::DEFAULT_COST,
        )
        .expect("Failed to hash password");

        let result = diesel::update(user.filter(id.eq(user_object.id)))
            .set((
                pseudo.eq(user_object.pseudo.as_ref()),
                email.eq(user_object.email.as_ref()),
                password_hash.eq(Some(hashed_password)),
                role.eq(user_object.role.as_ref()),
            ))
            .returning((id, pseudo, email, password_hash, role))
            .get_result::<UserObject>(self.conn);

        match result {
            Ok(user_object) => Ok(user_object),
            Err(e) => Err(e),
        }
    }

    fn login(&mut self, p_email: &str, p_password: &str) -> Result<String, Error> {
        use crate::schema::user::dsl::*;

        // Load environment variables from .env file
        dotenv().ok();
        let secret_key = env::var(
            "
        JWT_SECRET",
        )
        .expect("JWT_SECRET must be set");

        let result = user
            .filter(email.eq(p_email))
            .select((id, pseudo, email, password_hash, role))
            .first::<UserObject>(self.conn);

        match result {
            Ok(user_object) => {
                if bcrypt::verify(p_password, user_object.password_hash.as_ref().unwrap())
                    .expect("Failed to verify password")
                {
                    let claims = Claims {
                        sub: user_object.email.unwrap(),
                        exp: 10000000000, // Set expiration as needed
                    };
                    let token = encode(
                        &Header::default(),
                        &claims,
                        &EncodingKey::from_secret(secret_key.as_ref()),
                    )
                    .map_err(|_| Error::NotFound)?;
                    Ok(token)
                } else {
                    Err(Error::NotFound)
                }
            }
            Err(e) => Err(e),
        }
    }
}
