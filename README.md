# actix-web-multipart-test
=======

A helper for creating `multipart/form-data` payloads for `actix-web` tests.

This crate provides a `TestMultipartBuilder` to simplify the process of creating
`multipart/form-data` payloads for integration tests in Actix-Web applications.
It allows you to easily add text, JSON, and binary parts to your test requests.

## Usage

```rust
use actix_web::{App, Responder, post, test};
use actix_multipart::form::{
    json::Json as MpJson, tempfile::TempFile, MultipartForm, MultipartFormConfig,
};
use serde::{Deserialize, Serialize};
use actix_web_multipart_test::TestMultipartBuilder;

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    name: String,
}

#[derive(Debug, MultipartForm)]
struct UploadForm {
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
    let app = test::init_service(
        App::new()
            .service(post_video)
            .app_data(MultipartFormConfig::default().total_limit(100 * 1024 * 1024)),
    )
    .await;

    // Arrange
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
```

## See also

The official [`actix-multipart`](https://crates.io/crates/actix-multipart) crate already offers helper functions to create multi-part requests in tests, namely [`create_form_data_payload_and_headers`](https://docs.rs/actix-multipart/0.7.2/actix_multipart/test/fn.create_form_data_payload_and_headers.html) and [`create_form_data_payload_and_headers_with_boundary`](https://docs.rs/actix-multipart/0.7.2/actix_multipart/test/fn.create_form_data_payload_and_headers_with_boundary.html). These might already cover your use-case and therefore not warrant pulling in another dependency.

There is also another similar [`actix-multipart-test`](https://github.com/jonatansalemes/actix-multipart-test) crate which seems rather
unmaintained though.

---

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>