mod integration;

use integration::notion::member::get_members_id;

#[tokio::main]
async fn main() {
    get_members_id().await;
}
