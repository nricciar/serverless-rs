use actix::prelude::*;
use actix_web::*;
use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::pg::upsert::*;

use models;
use schema;

pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

pub struct CreateLambda {
    pub path: String,
    pub hostname: String,
    pub code: String,
}

pub struct GetLambda {
    pub path: String,
    pub hostname: String,
}

impl Message for CreateLambda {
    type Result = Result<models::Lambda, Error>;
}

impl Message for GetLambda {
    type Result = Result<models::Lambda, Error>;
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

impl Handler<CreateLambda> for DbExecutor {
    type Result = Result<models::Lambda, Error>;

    fn handle(&mut self, msg: CreateLambda, _: &mut Self::Context) -> Self::Result {
        use self::schema::lambdas::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap(); 

        let new_lambda = models::NewLambda {
            path: &msg.path,
            hostname: &msg.hostname,
            code: &msg.code,
        };

        let ret = diesel::insert_into(lambdas)
            .values(&new_lambda)
            .on_conflict((hostname, path))
            .do_update()
            .set(code.eq(new_lambda.code))
            .get_result(conn)
            .map_err(|_| error::ErrorInternalServerError("Error inserting lambda"))?;

        Ok(ret)
    }
}

impl Handler<GetLambda> for DbExecutor {
    type Result = Result<models::Lambda, Error>;

    fn handle(&mut self, msg: GetLambda, _: &mut Self::Context) -> Self::Result {
        use self::schema::lambdas::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();

        let mut items = lambdas
            .filter(path.eq(msg.path))
            .filter(hostname.eq(msg.hostname))
            .load::<models::Lambda>(conn)
            .map_err(|_| error::ErrorInternalServerError("Error loading lambda"))?;

        Ok(items.pop().unwrap())
    }
}