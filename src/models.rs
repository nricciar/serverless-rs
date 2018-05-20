use super::schema::lambdas;

#[derive(Serialize, Queryable)]
pub struct Lambda {
    pub id: i32,
    pub path: String,
    pub hostname: String,
    pub code: String,
}

#[derive(Insertable)]
#[table_name = "lambdas"]
pub struct NewLambda<'a> {
    pub path: &'a str,
    pub hostname: &'a str,
    pub code: &'a str,
}
