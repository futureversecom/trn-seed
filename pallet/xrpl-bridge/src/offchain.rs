use serde_json::{json, to_vec};
use sp_runtime::offchain::http::Request;
use sp_runtime::offchain::{Duration, http};
use sp_std::vec::Vec;
use sp_std::vec;

pub fn get_xrpl_block_data() -> Result<(), http::Error> {
    // Temp values for testing
    let tx_hash = "CAECA8C9DE80AE296D260FD86A4233D38E9DE9E749AFE4967BCE41533443B114";
    let ledger_index = 72014720;

    let body = rpc_body("transaction_entry", tx_hash, ledger_index);

    make_rpc_call("https://s1.ripple.com:51234/", body)
}

// Build RPC body for getting tx by ledger
fn rpc_body(method: &str, tx_hash: &str, ledger_index: u64) -> Vec<u8> {
    let body = json!({
        "method": method,
        "params": [
            {
                "tx_hash": tx_hash,
                "ledger_index": ledger_index
            }
        ]
    });
    to_vec(&body).unwrap()
}

fn make_rpc_call(url: &str, body: Vec<u8>) -> Result<(), http::Error>  {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

    let pending = Request::new(url)
        .method(http::Method::Post)
        .body(vec![body])
        .add_header("Content-Type", "application/json")
        .send()
        .unwrap();

    // let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
    
    let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
    // Let's check the status code before we proceed to reading the response.
    if response.code != 200 {
        log::warn!("Unexpected status code: {}", response.code);
        return Err(http::Error::Unknown)
    }

    let body = response.body().collect::<Vec<u8>>();

    // Create a str slice from the body.
    let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
        log::warn!("No UTF8 body");
        http::Error::Unknown
    })?;

    log::info!("body {:?}", body_str);


    Ok(())
}

