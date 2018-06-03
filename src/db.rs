use actix::prelude::*;
use actix_web::*;
use diesel;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use actix::prelude::{Addr,Syn};

use models;
use schema;
use request;

pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

pub struct AppState {
    pub db: Addr<Syn, DbExecutor>,
}

pub struct CreateLambda {
    pub path: String,
    pub hostname: String,
    pub code: String,
}

pub struct GetLambda {
    pub request: request::Request,
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
            .filter(path.eq(msg.request.path()))
            .filter(hostname.eq(msg.request.host()))
            .load::<models::Lambda>(conn)
            .map_err(|_| error::ErrorInternalServerError("Error loading lambda"))?;

        match items.pop() {
            Some(i) => Ok(i),
            None => Err(error::ErrorNotFound("Not Found")),
        }
    }
}