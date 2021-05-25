use actix_web::{web, guard, App, HttpServer, HttpResponse, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql::*;
use async_graphql_actix_web::{Request, Response};
use mongodb::{Client, options::ClientOptions};
use serde::{Deserialize, Serialize};
use futures::stream::TryStreamExt;
use futures::stream::{self, StreamExt};
use mongodb::{bson::doc, bson::DateTime, options::FindOptions};

#[derive(Debug, Serialize, Deserialize, SimpleObject)]
struct User {
    name: String,
    email: String,
    image: String,
    rating: f32
}

struct QueryRoot;
#[Object]
impl QueryRoot {
    async fn user(&self, ctx: &Context<'_>) -> Vec<User> {
        // Look up users from the database
        let db_instance = ctx.data::<mongodb::Database>().expect("error");
        let cursor = db_instance.collection::<User>("users");
        let mut users = cursor.find(doc! {}, None).await.expect("query error");
        let mut results = vec![];

        while let Some(user) = users.try_next().await.expect("error") {
            results.push(user);
        }
        results
    }
}

async fn index(data: web::Data<AppState>, req: Request) -> Response {
    let mut request = req.into_inner();
    request = request.data(data.db.clone());
    data.schema.execute(request).await.into()
}

async fn index_playground() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/graphql"),
    )))
}

struct AppState {
    db: mongodb::Database,
    schema: async_graphql::Schema<QueryRoot, async_graphql::EmptyMutation, async_graphql::EmptySubscription>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client_uri = "mongodb://127.0.0.1:27017";
    let options = ClientOptions::parse(&client_uri).await.expect("client error");
    let client = Client::with_options(options).expect("client error");
    let db = client.database("colony");

    // let typed_collection = db.collection::<User>("users");
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();

    HttpServer::new(move || {
        App::new()
        .data(AppState {
            db: db.clone(),
            schema: schema.clone()
        })
        .service(web::resource("/graphql").guard(guard::Get()).to(index_playground))
        .service(web::resource("/graphql").guard(guard::Post()).to(index))
    }).bind("127.0.0.1:3000").expect("HTTP error.")
    .run()
    .await
}
