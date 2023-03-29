use flume::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::{net::{TcpSocket, TcpStream}, io::{BufReader, AsyncBufReadExt, AsyncWriteExt}, task::spawn_blocking};
use std::env;

struct PrimeCheckInfo {
    check_prime: i64,
    return_ch: tokio::sync::oneshot::Sender<bool>
}

// {"method":"isPrime","number":123}

#[derive(Debug, Serialize, Deserialize)]
struct RequestType {
    method: String,
    number: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorType {
    error: String,
}

// {"method":"isPrime","prime":false}
#[derive(Debug, Serialize, Deserialize)]
struct ResponseType {
    method: String,
    prime: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Prime Generator");
    let args: Vec<String> = env::args().collect();
    let port = args[1].parse::<i32>()?;


    let addr = format!("0.0.0.0:{port}").parse()?;
    let socket = TcpSocket::new_v4()?;
    socket.bind(addr)?;
    socket.reuseaddr()?;
    let listener = socket.listen(1024)?;

    let (prime_check_sx, prime_check_rx) = flume::unbounded::<PrimeCheckInfo>();

    for _ in 0..1 {
        let rx = prime_check_rx.clone();
        spawn_blocking(move || {
            check_primes(rx)
        });

    }

    loop {
        let (c, _) = listener.accept().await?;

        let sx = prime_check_sx.clone();
        tokio::spawn(async move {
            let rc = handle_client(c, sx).await;
            let _ = dbg!(rc);
        });

    }
}


async fn handle_client(c: TcpStream, check_ch: Sender<PrimeCheckInfo>) -> anyhow::Result<()> {

    println!("handle client");
    let mut buf_reader = BufReader::new(c);
    let mut line = String::new();
    loop  {
        line.clear();

        let num_bytes = buf_reader.read_line(&mut line).await?;

        if num_bytes == 0 {
            break;
        }
        dbg!(&line);
        let json_req : Result<RequestType, _> = serde_json::from_str(&line);
        dbg!(&json_req);
        if json_req.is_err() {
            let error_string = json_req.err().unwrap().to_string();
            let error_json = serde_json::to_string(&ErrorType{error: error_string})?;
            buf_reader.write_all(dbg!(error_json.as_bytes())).await?;
            buf_reader.write_u8(b'\n').await?;
            break;
        }
        let json_req = json_req.unwrap();

        if json_req.method != "isPrime" {
            let error_json = serde_json::to_string(&ErrorType{error:  "only isPrime is supported as method".to_owned()})?;
            buf_reader.write_all(dbg!(error_json.as_bytes())).await?;
            buf_reader.write_u8(b'\n').await?;
            break;
        }
        dbg!(&json_req.number);

        let mut ans = false;
        if f64::abs(json_req.number.trunc() - json_req.number) < f64::EPSILON {
            let (sx, rx) = tokio::sync::oneshot::channel();

            check_ch.send_async(PrimeCheckInfo{check_prime: json_req.number.trunc() as i64, return_ch: sx}).await?;
            ans = rx.await?;
        }

        dbg!(&ans);
        let response_json = serde_json::to_string(&ResponseType{method: json_req.method, prime : ans})?;
        dbg!(&response_json);

        buf_reader.write_all(response_json.as_bytes()).await?;
        buf_reader.write_u8(b'\n').await?;
    }
    Ok(())
}

fn setup_prime_set(prime_set : &mut bit_set::BitSet, old_max: usize, new_max: usize) {
    for i in old_max..new_max{
        prime_set.insert(i);
    }

    for i in 2.. (new_max / 2 + 1) {
            for j in 2..(new_max / i + 1) {
            prime_set.remove(i*j);
        }
    }
}

fn check_primes(in_ch: Receiver<PrimeCheckInfo>) -> anyhow::Result<()> {
    let max_prime = 100000000;
    let mut prime_set = bit_set::BitSet::new();
    setup_prime_set(&mut prime_set, 0, max_prime);
    println!("Prime Setup done");

    loop {

        let m = in_ch.recv()?;
        let p = m.check_prime;
        if p <= 1 {
            let _ = m.return_ch.send(false);
            continue;
        }

        let p = p as usize;
        if p < max_prime {
            let ans = prime_set.contains(p);
            let _ = m.return_ch.send(ans);
            continue;
        }

        let mut ans = true;
        for i in 2.. (p/ 2 + 1) {
            if p % i == 0 {
                ans = false;
                break;
            }
        }
        let _ = m.return_ch.send(ans);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prime_set() {
        let mut prime_set = bit_set::BitSet::new();
        setup_prime_set(&mut prime_set, 0, 100);

        assert!(!prime_set.contains(6));
        assert!(!prime_set.contains(99));
        assert!(prime_set.contains(7));
        assert!(prime_set.contains(97));
    }
}
