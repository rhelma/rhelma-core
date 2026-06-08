rust
use async_graphql::{Schema, Context, FieldResult, Object, Query, Field, ID};
use async_graphql::http::{playground_source};
use async_graphql::Schema;

#[derive(Default)]
struct Query;

#[Object]
impl Query {
    async fn node(&self, ctx: &Context<'_>, id: ID) -> FieldResult<String> {
        Ok(format!("Node ID: {}", id))
    }
}

async fn start_graphql_server() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .finish();

    let addr = "127.0.0.1:4000".parse()?;
    let service = warp::path("graphql")
        .and(async_graphql_warp::graphql(schema))
        .map(|response: async_graphql::Response| warp::reply::json(&response));
    
    warp::serve(service).run(addr).await;

    Ok(())
}
