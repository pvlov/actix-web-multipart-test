//! A helper for creating `multipart/form-data` payloads for Actix-Web tests.
//!
//! This crate provides a `TestMultipartBuilder` to simplify the process of creating
//! `multipart/form-data` payloads for integration tests in Actix-Web applications.
//! It allows you to easily add text, JSON, and binary parts to your test requests.
//!
//! ## Usage
//!
//! ```rust
//! # use actix_web::{App, Responder, post, test};
//! # use actix_multipart::form::{
//! #     json::Json as MpJson, tempfile::TempFile, MultipartForm, MultipartFormConfig,
//! # };
//! # use serde::{Deserialize, Serialize};
//! # use actix_web_multipart_test::TestMultipartBuilder;
//! #
//! # #[derive(Debug, Deserialize, Serialize)]
//! # struct Metadata {
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, MultipartForm)]
//! # struct UploadForm {
//! #     #[multipart(limit = "100MB")]
//! #     file: TempFile,
//! #     json: MpJson<Metadata>,
//! # }
//! #
//! # #[post("/videos")]
//! # async fn post_video(MultipartForm(form): MultipartForm<UploadForm>) -> impl Responder {
//! #     format!(
//! #         "Uploaded file {}, with size: {}\ntemporary file ({}) was deleted\n",
//! #         form.json.name,
//! #         form.file.size,
//! #         form.file.file.path().display(),
//! #     )
//! # }
//! #
//! # #[actix_web::test]
//! # async fn test_builder_against_handler() {
//! #     let app = test::init_service(
//! #         App::new()
//! #             .service(post_video)
//! #             .app_data(MultipartFormConfig::default().total_limit(100 * 1024 * 1024)),
//! #     )
//! #     .await;
//! #
//! // Arrange
//! let metadata = Metadata {
//!     name: "MyTestVideo".to_string(),
//! };
//! let file_content = "This is a dummy video file".as_bytes();
//!
//! let builder = TestMultipartBuilder::new()
//!     .with_json("json", &metadata)
//!     .with_bytes(
//!         "file",
//!         "test_video.mp4",
//!         "video/mp4",
//!         file_content,
//!     );
//!
//! let (content_type, body) = builder.build();
//!
//! // Act
//! let req = test::TestRequest::post()
//!     .uri("/videos")
//!     .insert_header(content_type)
//!     .set_payload(body)
//!     .to_request();
//!
//! let resp = test::call_service(&app, req).await;
//!
//! // Assert
//! assert!(resp.status().is_success(), "Response was not 2xx");
//!
//! let body_bytes = test::read_body(resp).await;
//! let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
//!
//! assert!(body_str.contains("MyTestVideo"));
//! assert!(body_str.contains(&file_content.len().to_string()));
//! assert!(body_str.contains("was deleted"));
//! # }
//! ```
//!
//! ## Features
//!
//! - `json`: Enables the `with_json` method to add `application/json` parts from serializable data.
//!
use actix_web::http::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use bytes::{Bytes, BytesMut};
use uuid::Uuid;

#[cfg(feature = "json")]
use serde::Serialize;

#[cfg(feature = "json")]
use serde_json;

/// A builder for creating `multipart/form-data` payloads for Actix-Web tests.
pub struct TestMultipartBuilder {
    boundary: String,
    parts: Vec<Part>,
}

/// Represents one part of the multipart payload.
struct Part {
    name: String,
    content_type: String,
    filename: Option<String>,
    content: Bytes,
}

impl TestMultipartBuilder {
    /// Create a new builder with a random boundary.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            boundary: Uuid::new_v4().to_string(),
            parts: Vec::new(),
        }
    }

    /// Add a simple text part (e.g., "text/plain").
    pub fn with_text(self, name: &str, text: &str) -> Self {
        self.with_part(
            name.to_string(),
            "text/plain".to_string(),
            None,
            Bytes::from(text.to_string()),
        )
    }

    /// Add a part from in-memory bytes (e.g., a file).
    pub fn with_bytes(
        self,
        name: &str,
        filename: &str,
        content_type: &str,
        content: impl Into<Bytes>,
    ) -> Self {
        self.with_part(
            name.to_string(),
            content_type.to_string(),
            Some(filename.to_string()),
            content.into(),
        )
    }
    
    /// Add a serializable JSON part with "application/json".
    ///
    /// This method is only available when the `json` feature is enabled.
    #[cfg(feature = "json")]
    pub fn with_json<T: Serialize>(self, name: &str, json_data: &T) -> Self {
        let content = serde_json::to_vec(json_data).unwrap();
        self.with_part(
            name.to_string(),
            "application/json".to_string(),
            None, // JSON parts typically don't have a filename
            Bytes::from(content),
        )
    }

    /// The generic "add part" method.
    pub fn with_part(
        mut self,
        name: String,
        content_type: String,
        filename: Option<String>,
        content: Bytes,
    ) -> Self {
        self.parts.push(Part {
            name,
            content_type,
            filename,
            content,
        });
        self
    }

    /// Build the final (HeaderValue, Bytes) tuple for the test request.
    pub fn build(self) -> ((HeaderName, HeaderValue), Bytes) {
        let mut body = BytesMut::new();

        for part in self.parts {
            body.extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
            
            let disposition = if let Some(filename) = part.filename {
                format!(
                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                    part.name, filename
                )
            } else {
                format!("Content-Disposition: form-data; name=\"{}\"\r\n", part.name)
            };
            body.extend_from_slice(disposition.as_bytes());

            body.extend_from_slice(
                format!("Content-Type: {}\r\n\r\n", part.content_type).as_bytes(),
            );
            body.extend_from_slice(&part.content);
            body.extend_from_slice("\r\n".as_bytes());
        }

        body.extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());

        let content_type_value =
            HeaderValue::from_str(&format!("multipart/form-data; boundary={}", self.boundary))
                .unwrap();

        ((CONTENT_TYPE, content_type_value), body.freeze())
    }
}

impl Default for TestMultipartBuilder {
    
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use actix_multipart::form::{
        json::Json as MpJson, tempfile::TempFile, MultipartForm, MultipartFormConfig,
    };
    use actix_web::{App, Responder, post, test};
    use serde::{Deserialize, Serialize};

    use super::TestMultipartBuilder;


    #[derive(Debug, Deserialize, Serialize)]
    struct Metadata {
        name: String,
    }

    #[derive(Debug, MultipartForm)]
    struct UploadForm {
        // Note: the form is also subject to the global limits configured using `MultipartFormConfig`.
        #[multipart(limit = "100MB")]
        file: TempFile,
        json: MpJson<Metadata>,
    }

    #[post("/videos")]
    async fn post_video(MultipartForm(form): MultipartForm<UploadForm>) -> impl Responder {
        format!(
            "Uploaded file {}, with size: {}\ntemporary file ({}) was deleted\n",
            form.json.name,
            form.file.size,
            form.file.file.path().display(),
        )
    }

    #[actix_web::test]
    async fn test_builder_against_handler() {
        // Arrange
        let app = test::init_service(
            App::new()
                .service(post_video)
                .app_data(MultipartFormConfig::default().total_limit(100 * 1024 * 1024)),
        )
        .await;

        let metadata = Metadata {
            name: "MyTestVideo".to_string(),
        };
        let file_content = "This is a dummy video file".as_bytes();

        let builder = TestMultipartBuilder::new()
            .with_json("json", &metadata)
            .with_bytes(
                "file",
                "test_video.mp4",
                "video/mp4",
                file_content,
            );

        let (content_type, body) = builder.build();

        // Act
        let req = test::TestRequest::post()
            .uri("/videos")
            .insert_header(content_type)
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Assert
        assert!(resp.status().is_success(), "Response was not 2xx");

        let body_bytes = test::read_body(resp).await;
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(body_str.contains("MyTestVideo"));
        assert!(body_str.contains(&file_content.len().to_string()));
        assert!(body_str.contains("was deleted"));
    }
}