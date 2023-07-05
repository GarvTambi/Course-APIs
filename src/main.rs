use lambda_http::{lambda_runtime::Context};
use lambda_http::{
    Request, Response, Body, RequestExt,
    http::header::{HeaderValue, CONTENT_TYPE},
};
use rusoto_core::{Region, RusotoError};
use rusoto_dynamodb::{
    AttributeValue, DynamoDb, DynamoDbClient, GetItemError, GetItemInput, GetItemOutput, PutItemInput, QueryError, QueryInput,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tokio::runtime::Runtime;

#[derive(Debug, Deserialize, Serialize)]
struct Course {
    course_id: String,
    course_name: String,
    course_category: String,
}

async fn create_course(course: Course) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let client = DynamoDbClient::new(Region::default());
    let rt = Runtime::new()?;

    let mut item = HashMap::new();
    item.insert("course_id".to_string(), AttributeValue {
        s: Some(course.course_id),
        ..Default::default()
    });
    item.insert("course_name".to_string(), AttributeValue {
        s: Some(course.course_name),
        ..Default::default()
    });
    item.insert("course_category".to_string(), AttributeValue {
        s: Some(course.course_category),
        ..Default::default()
    });

    let input = PutItemInput {
        item,
        table_name: "Courses".to_string(),
        ..Default::default()
    };

    let response = rt.block_on(client.put_item(input));
    match response {
        Ok(_) => Ok(Response::builder()
            .status(200)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(Body::from("{\"message\":\"Course created successfully.\"}"))
            .unwrap()),
        Err(err) => {
            println!("Create course error: {:?}", err);
            Err(Box::new(err))
        }
    }
}

async fn get_item(
    client: &DynamoDbClient,
    input: GetItemInput,
) -> Result<GetItemOutput, RusotoError<GetItemError>> {
    client.get_item(input).await
}


async fn get_course_by_id(course_id: String) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let client = DynamoDbClient::new(Region::default());

    let input = GetItemInput {
        key: {
            let mut key = HashMap::new();
            key.insert("course_id".to_string(), AttributeValue {
                s: Some(course_id),
                ..Default::default()
            });
            key
        },
        table_name: "Courses".to_string(),
        ..Default::default()
    };

    let response = get_item(&client, input).await;
    match response {
        Ok(output) => {
            if let Some(item) = output.item {
                let course = Course {
                    course_id: item.get("course_id").unwrap().s.as_ref().unwrap().clone(),
                    course_name: item.get("course_name").unwrap().s.as_ref().unwrap().clone(),
                    course_category: item.get("course_category").unwrap().s.as_ref().unwrap().clone(),
                };

                Ok(Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from(serde_json::to_string(&course)?))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(404)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from("{\"message\":\"Course not found.\"}"))
                    .unwrap())
            }
        }
        Err(err) => {
            if let RusotoError::Service(GetItemError::ResourceNotFound(_)) = &err {
                Ok(Response::builder()
                    .status(404)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from("{\"message\":\"Course not found.\"}"))
                    .unwrap())
            } else {
                println!("Get course error: {:?}", err);
                Err(Box::new(err))
            }
        }
    }
}


async fn get_courses_by_category(course_category: String) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let client = DynamoDbClient::new(Region::default());

    let expression_attribute_values = {
        let mut values = HashMap::new();
        values.insert(":category".to_string(), AttributeValue {
            s: Some(course_category.clone()),
            ..Default::default()
        });
        values
    };

    let expression_attribute_names = {
        let mut names = HashMap::new();
        names.insert("#category".to_string(), "course_category".to_string());
        names
    };

    let filter_expression = Some("#category = :category".to_string());

    let input = QueryInput {
        table_name: "Courses".to_string(),
        key_condition_expression: Some("#category = :category".to_string()),
        expression_attribute_values: Some(expression_attribute_values),
        expression_attribute_names: Some(expression_attribute_names),
        filter_expression,
        ..Default::default()
    };

    match client.query(input).await {
        Ok(output) => {
            if let Some(item) = output.items {
                // Process the query output and return the response
                // Rest of the code...
                Ok(Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from(serde_json::to_string(&item)?))
                    .unwrap())
            } else {
                Ok(Response::builder()
                    .status(404)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from("{\"message\":\"No courses found in the specified category.\"}"))
                    .unwrap())
            }
        }
        Err(err) => {
            if let RusotoError::Service(QueryError::ResourceNotFound(_)) = &err {
                Ok(Response::builder()
                    .status(404)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from("{\"message\":\"No courses found in the specified category.\"}"))
                    .unwrap())
            } else {
                println!("Get courses by category error: {:?}", err);
                Err(Box::new(err))
            }
        }
    }
}


async fn router(req: Request, _: lambda_http::Context) -> Result<Response<Body>, lambda_http::Error> {
    match (req.method().as_str(), req.uri().path()) {
        ("POST", "/courses") => {
            let course: Course = serde_json::from_slice(req.body().as_ref())?;
            create_course(course).await
        }
        ("GET", "/courses") => {
            let query_params = req.query_string_parameters();
            let course_id = query_params.get("course_id").map(|value| value.iter().next().map(|value| value.to_string()).unwrap_or_default());
            let course_category = query_params.get("course_category").map(|value| value.iter().next().map(|value| value.to_string()).unwrap_or_default());

            if let Some(course_id) = course_id {
                get_course_by_id(course_id).await
            } else if let Some(course_category) = course_category {
                get_courses_by_category(course_category).await
            } else {
                Ok(Response::builder()
                    .status(400)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                    .body(Body::from("{\"message\":\"Invalid request.\"}"))
                    .unwrap())
            }
        }
        _ => Ok(Response::builder()
            .status(404)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(Body::from("{\"message\":\"Not found.\"}"))
            .unwrap()),
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _function_name = env::var("AWS_LAMBDA_FUNCTION_NAME").unwrap_or_else(|_| "local".to_string());
    let service = lambda_http::service_fn(move |req: Request| {
        let ctx = Context::clone(&req.extensions().get::<Context>().unwrap());
        router(req, ctx)
    });
    let _ = lambda_http::run(service).await;
    Ok(())
}
