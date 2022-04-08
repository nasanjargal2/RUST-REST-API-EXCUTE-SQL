#[macro_use]
extern crate diesel;

use actix_web::http::header::IntoHeaderValue;
#[macro_use]
use actix_web::{
    App, HttpServer, HttpMessage, HttpRequest, HttpResponse, web, get, put, post, delete,
    ResponseError,
};
use crate::diesel::{Insertable, Queryable};
use crate::schema::users::dsl::*;
use crate::schema::users::{self, displayname};
use actix_web::http::StatusCode;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::query_builder::{AsQuery, QueryBuilder};
use diesel::query_dsl::QueryDsl;
use diesel::result::Error as DieselError;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fmt;
use std::result::Result;
mod schema;
use diesel::dsl::count;
use diesel::dsl::{count_star, sql, Limit};
use diesel::pg::PgQueryBuilder;
use diesel::query_dsl::RunQueryDsl;
use diesel::sql_query;
//----------------------------------------------------DB холболт-------------------------------------------------------//
//----------------------------------------------------DB Connection----------------------------------------------------//

fn conn() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL-ыг тохируулах шаардлагатай!");
    PgConnection::establish(&database_url)
        .expect(&format!("Холбох үед алдаа гарлаа: {}", database_url))
}

//-----------------------------------------------------STRUCT зарлалт-----------------------------------------------------//
//-----------------------------------------------------Үндсэн STRUCT------------------------------------------------------//

#[derive(Debug, Deserialize, Serialize, Queryable, QueryableByName, Clone)]
#[table_name = "users"]
pub struct Users {
    pub id: i32,
    pub username: String,
    pub displayname: String,
    pub descriptions: String,
}

// DB руу шивэх STRUCT
#[derive(Insertable, Deserialize, Serialize)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub displayname: String,
    pub descriptions: String,
}

// DB д байгаа table өөрчлалт хийх STRUCT
#[derive(Debug, AsChangeset, Deserialize, Serialize, QueryableByName)]
#[table_name = "users"]
pub struct UpdateUser {
    pub username: String,
    pub displayname: String,
    pub descriptions: String,
}

//QWERY SELECT ORDER OFFSET FILTER LIMIT STRUCT
#[derive(Debug, Deserialize, Serialize)]
pub struct QueryInfo {
    //  pub select: Option<String>,
    pub order: Option<String>,
    pub offset: Option<i64>,    // page count
    pub filter: Option<String>, // where
    pub limit: Option<i64>,     // elements of one page
    pub total_count: Option<bool>,
}
//QWERY COUNT LIMIT
#[derive(Serialize, Deserialize, Debug)]

pub struct CustomStruct {
    pub total_count: Option<i64>,
    // pub filter: Option<String>,\         // where
    pub has_more: bool,

    pub items: Vec<Users>,
}

//-----------------------------------------------------ERROR--------------------------------------------------------------//
//-----------------------------------------------------Алдаа--------------------------------------------------------------//
#[derive(Debug, Deserialize)]
pub struct CustomError {
    pub error_status_code: u16,
    pub error_message: String,
}

impl CustomError {
    pub fn new(error_status_code: u16, error_message: String) -> CustomError {
        CustomError {
            error_status_code,
            error_message,
        }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.error_message.as_str())
    }
}
//-----------------------------------------------------DIESEL ERROR------------------------------------------------------//
//-----------------------------------------------------DIESL Алдаа-------------------------------------------------------//
impl From<DieselError> for CustomError {
    fn from(error: DieselError) -> CustomError {
        match error {
            DieselError::DatabaseError(_, err) => CustomError::new(409, err.message().to_string()),
            DieselError::NotFound => CustomError::new(404, "Хэрэглэгч олдсонгүй".to_string()),
            err => CustomError::new(500, format!("Unknown Diesel error: {}", err)),
        }
    }
}

impl ResponseError for CustomError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match StatusCode::from_u16(self.error_status_code) {
            Ok(status_code) => status_code,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_message = match status_code.as_u16() < 500 {
            true => self.error_message.clone(),
            false => "Internal server error".to_string(),
        };

        HttpResponse::build(status_code).json(json!({ "message": error_message }))
    }
}

//--------------------------------------ROUTES-------------------------------------------------------------------------------//
//--------------------------------------ROUTES-------------------------------------------------------------------------------//

//--------------------------------------ID гаар шүүх--------------------------------------------------------------//
//--------------------------------------Find for ID---------------------------------------------------------------//

#[get("/users/{id}")]
async fn find(i: web::Path<i32>) -> Result<HttpResponse, CustomError> {
    let user = Users::find(i.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

//--------------------------------------Бүх хэрэглэгч шүүх-----------------------LIMIT OFFSETS-----------------------------//
//--------------------------------------Find for Users---------------------------LIMIT OFFSETS-----------------------------//

#[get("/users")]
async fn find_all(req: web::Query<QueryInfo>) -> Result<HttpResponse, CustomError> {
    let user = Users::find_all(req.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

//--------------------------------------User  post to DB-----------------------------------------------------------//
//--------------------------------------ДБ руу Хэрэглэгч шивэх-----------------------------------------------------//

#[post("/users")]
async fn create(user: web::Json<NewUser>) -> Result<HttpResponse, CustomError> {
    let user = Users::create(user.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

//--------------------------------------ID гаар Хэрэглэгч засах---------------------------------------------------//
//--------------------------------------Edit user on ID-----------------------------------------------------------//

#[put("/users/{id}")]
async fn update(
    i: web::Path<i32>,
    user: web::Json<UpdateUser>,
) -> Result<HttpResponse, CustomError> {
    let user = Users::update(i.into_inner(), user.into_inner())?;
    Ok(HttpResponse::Ok().json(user))
}

//--------------------------------------ID гаар Хэрэглэгч устгах-------------------------------------------------//
//--------------------------------------Delete user on ID--------------------------------------------------------//

#[delete("/users/{id}")]
async fn delete(i: web::Path<i32>) -> Result<HttpResponse, CustomError> {
    let deleted_user = Users::delete(i.into_inner())?;
    Ok(HttpResponse::Ok().json(json!({ "deleted": deleted_user })))
}

pub fn init_routes(config: &mut web::ServiceConfig) {
    config.service(find_all);
    config.service(create);
    config.service(find);
    config.service(update);
    config.service(delete);
}

//--------------------------------------Удирдлага------------------------------------------------------------------------------//
//--------------------------------------CONTROLLER-----------------------------------------------------------------------------//

impl Users {
    //--------------------------------------User post to DB-----------------------------------------------------------//
    //--------------------------------------ДБ руу Хэрэглэгч шивэх----------------------------------------------------//

    pub fn create(user: NewUser) -> Result<Self, CustomError> {
        let conn = conn();
        let user = NewUser::from(user);
        let user = diesel::insert_into(users).values(user).get_result(&conn)?;
        Ok(user)
    }

    //--------------------------------------ID гаар шүүх--------------------------------------------------------------//
    //--------------------------------------Find for ID---------------------------------------------------------------//

    pub fn delete(i: i32) -> Result<usize, CustomError> {
        let conn = conn();
        let res = diesel::delete(users.filter(id.eq(i))).execute(&conn)?;
        Ok(res)
    }

    //--------------------------------------Бүх хэрэглэгч шүүх----------------------------------------------------------//
    //--------------------------------------Find for Users--------------------------------------------------------------//

    pub fn find(i: i32) -> Result<Self, CustomError> {
        let conn = conn();
        let user = users.filter(id.eq(i)).first(&conn)?;
        Ok(user)
    }

    pub fn find_all(req: QueryInfo) -> Result<CustomStruct, CustomError> {
        let conn = conn();
        let mut filter_field = &req.filter;
        //
        let mut table = String::from("SELECT * FROM Users ");
        if req.filter.is_some() {
            table.push_str(" WHERE ");
            table.push_str(&req.filter.as_ref().unwrap().to_string());
        }
        //ORDER
        if req.order.is_some() {
            table.push_str("ORDER BY ");
            table.push_str(&req.order.unwrap().to_string());
        }

        //total_count
        let mut total_count: Option<i64> = None;
        if req.total_count.is_some() {
            if req.filter.is_some() {
                total_count = Some(
                    users::table
                        .filter(sql(&req.filter.as_ref().unwrap()))
                        .count()
                        .first::<i64>(&conn)
                        .unwrap(),
                );
            } else {
                total_count = Some(users::table.count().first::<i64>(&conn).unwrap());
            }
        }

        //offset
        if req.offset.is_some() {
            table.push_str(" OFFSET ");
            table.push_str(&req.offset.unwrap().to_string());
        }
        //
        //limit
        if req.limit.is_some() {
            table.push_str(" LiMIT ");
            let a = req.limit.unwrap() + 1;
            table.push_str(&a.to_string());
        }
        //
        //has more
        let mut has_more = false;
        let mut b = sql_query(&table).load::<Users>(&conn).unwrap();
        let mut d = Some(TryInto::try_into(b.len()).unwrap());
        if req.limit.is_some() {
            if d > req.limit {
                has_more = true;
                let mut zz = usize::try_from(req.limit.unwrap()).unwrap();
                b = b.get(0..zz).unwrap().to_vec();
            }
        }
        //
        let mut res = CustomStruct {
            total_count: total_count,
            has_more: has_more,
            items: b,
        };
        Ok(res)
    }
    //--------------------------------------ID гаар Хэрэглэгч засах---------------------------------------------------//
    //--------------------------------------Edit user on ID-----------------------------------------------------------//

    //--------------------------------------Бүх хэрэглэгч шүүх----------------------------------------------------//
    //--------------------------------------Find for Users--------------------------------------------------------//
    pub fn find_all_users() -> Result<Vec<Self>, CustomError> {
        let conn = conn();

        let user = users::table.load::<Users>(&conn)?;

        Ok(user)
    }

    //--------------------------------------ID гаар Хэрэглэгч устгах--------------------------------------------------//
    //--------------------------------------Delete user on ID---------------------------------------------------------//

    pub fn update(i: i32, user: UpdateUser) -> Result<Self, CustomError> {
        println!("user = {:?}", user);
        let conn = conn();
        let user = diesel::update(users)
            .filter(id.eq(i))
            .set(user)
            .get_result(&conn)?;
        Ok(user)
    }
}

impl NewUser {
    fn from(user: NewUser) -> NewUser {
        NewUser {
            username: user.username,
            displayname: user.displayname,
            descriptions: user.descriptions,
        }
    }
}

//<main>
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    //----------------------------------------------------Create connection-------------------------------------------------------//
    //----------------------------------------------------Create connection-------------------------------------------------------//

    conn();

    println!("\n\n\n\n------< RESULTS >------ \n");
    println!("\n{} {}", "DB-тэй амжилттай", "холбогдлоо",);
    dotenv::dotenv().expect("Тохиргооны .env файлыг уншиж чадсангүй!");
    let host: String = env::var("HOST").expect("HOST тохируулна уу!...");
    let port: String = env::var("PORT").expect("PORT тохируулна уу!...");

    println!("\n{} {}:{} дээр аслаа ...\n\n", "Server", host, port,);

    //----------------------------------------------------Start HTTP server--------------------------------------------------------//
    //----------------------------------------------------Start HTTP server--------------------------------------------------------//

    HttpServer::new(|| App::new().configure(init_routes))
        .bind(format!("{}:{}", host, port))?
        .run()
        .await
}
