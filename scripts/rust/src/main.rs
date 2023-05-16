
mod set_code;
use set_code::set_code;

#[tokio::main]
async fn main() {
    set_code().await.unwrap();
}