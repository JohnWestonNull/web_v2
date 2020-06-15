use actix_web::{HttpResponse, web};
use actix_web::body::Body;
use actix_web::Error;
use futures::future;
use futures::stream::StreamExt;
use futures_await_test::async_test;
use mongodb::bson::{Bson, doc, Document, from_bson};
use serde::{Deserialize, Serialize};

use crate::database::Database;

#[derive(Debug, Deserialize, Serialize)]
pub struct Course {
    cid: String,
    name: String,
    taught_by: Vec<Vec<String>>,
    faculty: String,
}

async fn get_course(db: Option<&Database>, filter: Option<Document>) -> Result<Vec<Course>, Box<dyn std::error::Error>> {
    use crate::database::DEFAULT_DATABASE;
    let db = db.unwrap_or(&*DEFAULT_DATABASE);
    let filter = doc! {"$match": filter.unwrap_or(doc!{})};
    let aggregator = doc! {
                "$group" : {
                    "_id" : "$cid",
                    "cid" : {"$first": "$cid"},
                    "name" : {"$first": "$name"},
                    "faculty" : {"$first": "$faculty"},
                    "taught_by" : {"$addToSet": "$taught_by"},
                }
            };
    Ok(db
        .connect()
        .await?
        .database(&db.name)
        .collection("Course")
        .aggregate(
            vec![filter, aggregator],
            None,
        )
        .await?
        .map(|d| {
            Ok::<Bson, mongodb::error::Error>(Bson::Document(d?))
        })
        .filter(|x| future::ready(Result::is_ok(x)))
        .map(|d| {
            from_bson::<Course>(d.unwrap())
        })
        .filter(|x| future::ready(Result::is_ok(x)))
        .map(|x| x.unwrap())
        .collect::<Vec<Course>>()
        .await
    )
}

async fn get_course_handler(req: web::Query<Bson>) -> Result<HttpResponse<Body>, Error> {
    let injson = match get_course(None, req.as_document().cloned()).await {
        Ok(v) => serde_json::to_string(&v),
        Err(e) => serde_json::to_string(&e.to_string()),
    };
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(injson.unwrap()))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/course")
            .route(web::get().to(get_course_handler))
    );
}

#[async_test]
async fn test_get_course_10_times() {
    for _ in 0..10 {
        get_course(None, None).await.unwrap();
    }
}


