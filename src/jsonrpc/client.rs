extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate spectral;

use self::reqwest::{Client as HTTPClient, Error as ResponseError};
use self::serde::de::{Deserialize, DeserializeOwned, Deserializer};
use self::serde::ser::{Serialize};

#[derive(Serialize)]
enum JsonRpcVersion {
    #[serde(rename = "1.0")]
    V1,

    #[serde(rename = "2.0")]
    V2,
}

#[derive(Serialize)]
struct Payload<T> where T: Serialize {
    jsonrpc: JsonRpcVersion,
    id: String,
    method: String,
    params: T,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Error {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Response<R> {
    Successful { id: String, result: R },
    Error { id: String, error: Error },
}

struct Client {
    client: HTTPClient,
    url: String,
}

impl Client {
    fn new(client: HTTPClient, url: &str) -> Self {
        Client {
            client,
            url: url.to_string(),
        }
    }
//
//    pub fn call0<E, R>(&self, id: &str, method: &str) -> Result<Response<R, E>, Error> where E: DeserializeOwned, R: DeserializeOwned {
//        self.call::<E, R, Vec<i32>>(id, method, vec![])
//    }
////
//    pub fn call1<'a, E, R, A>(&self, id: &str, method: &str, a: A) -> Result<Response<'a, R, E>, Error> where A: Serialize, E: DeserializeOwned, R: DeserializeOwned {
//        self.call(id, method, [a])
//    }
//
//    pub fn call2<'a, E, R, A, B>(&self, id: &str, method: &str, a: A, b: B) -> Result<Response<'a, R, E>, Error> where A: Serialize, B: Serialize, E: DeserializeOwned, R: DeserializeOwned {
//        self.call(id, method, (a, b))
//    }
//
//    fn call<E, R, Params>(&self, id: &str, method: &str, params: Params) -> Result<Response<R, E>, Error> where Params: Serialize, E: DeserializeOwned, R: DeserializeOwned {
//        let payload = Payload {
//            jsonrpc: JsonRpcVersion::V1,
//            id: id.to_string(),
//            method: method.to_string(),
//            params,
//        };
//
//        self.client
//            .post(self.url.as_str())
//            .json(&payload)
//            .send()
//            .and_then(|mut res| res.json::<Response<R, E>>())
//    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::spectral::prelude::*;

    #[test]
    fn can_serialize_payload_with_no_params() {
        let payload = Payload {
            jsonrpc: JsonRpcVersion::V1,
            id: "test".to_string(),
            method: "test".to_string(),
            params: (),
        };

        let expected_payload = r#"{"jsonrpc":"1.0","id":"test","method":"test","params":null}"#.to_string();

        let serialized_payload = serde_json::to_string(&payload).unwrap();

        assert_that(&serialized_payload).is_equal_to(expected_payload);
    }

    #[test]
    fn can_deserialize_successful_response_into_generic_type() {
        let result = r#"{
            "id": "test",
            "result": 519521,
            "error": null
        }"#;

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

        match deserialized_response {
            Response::Successful{id, result} => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&result).is_equal_to(519521);
            }
            Response::Error{id, error} => {
                panic!("Should not yield error")
            }
        }
    }

    #[test]
    fn can_deserialize_error_response() {
        let result = r#"{
            "id": "test",
            "result": null,
            "error": {
                "code": -123,
                "message": "Something went wrong"
            }
        }"#;

        let deserialized_response: Response<i32> = serde_json::from_str(result).unwrap();

        match deserialized_response {
            Response::Successful{id, result} => {
                panic!("Should not yield successful result");
            }
            Response::Error{id, error} => {
                assert_that(&id).is_equal_to("test".to_string());
                assert_that(&error.code).is_equal_to(-123);
                assert_that(&error.message).is_equal_to("Something went wrong".to_string());
            }
        }
    }
}