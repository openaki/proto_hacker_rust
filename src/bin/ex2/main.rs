use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::ops::Bound::Included;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, packed)]
struct Msg {
    c: u8,
    a: i32,
    b: i32,
}

fn parse_msg(buf: &[u8; 9]) -> Msg {
    //let mut buf : [u8; 9] = [0x51, 0, 0, 0x03, 0xe8,0,1,0x86, 0xa0];
    let buf_ptr = buf.as_ptr();
    let mut msg: Msg;
    unsafe {
        let m = buf_ptr as *const Msg;
        msg = *m;
        msg.a = msg.a.to_be();
        msg.b = msg.b.to_be();
    }
    msg
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let addr = "0.0.0.0:5002".parse::<SocketAddr>()?;
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.bind(addr)?;
    let tcp_listner = socket.listen(1024)?;

    loop {
        let (mut tcp_stream, _) = tcp_listner.accept().await?;
        tokio::spawn(async move {
            handle_tcp_stream(&mut tcp_stream).await
        });
    }

}

async fn handle_tcp_stream(tcp_stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf : [u8; 9] = [0; 9];
    let mut session = SessionHandler::default();
    loop {
        tcp_stream.read_exact(&mut buf).await?;
        let msg = parse_msg(&buf);
        if let Some(mean_price) = session.handle_msg(msg) {
            tcp_stream.write_i32(mean_price).await?;
        }
    }
}

#[derive(Debug, Default)]
struct SessionHandler {
    time_price_map : BTreeMap<i32, i32>,
}

impl SessionHandler {
    pub fn handle_msg(&mut self, msg: Msg) -> Option<i32>{
        match msg.c {
            b'I' => {
                self.time_price_map.insert(msg.a, msg.b);
                None
            },
            b'Q' => {
                let mut count = 0;
                let mut sum :i64 = 0;
                if msg.a > msg.b {
                    return Some(0);
                }
                self.time_price_map.range((Included(msg.a), Included(msg.b))).for_each(|(&_k, &v)| {
                    count += 1;
                    sum += v as i64;

                });
                if count == 0 {
                    Some(0)
                } else {
                    Some((sum / count) as i32)
                }

            },
            _ => None,
        }
    }

}


#[cfg(test)]
mod tests {
    use std::mem::transmute_copy;

    use super::*;

    fn msg_to_bytes(msg_i: &Msg) -> [u8; 9] {
        let mut msg : Msg = *msg_i;
        msg.a = msg.a.to_be();
        msg.b = msg.b.to_be();
        let buf : [u8; 9];
        unsafe {
            buf = transmute_copy(&msg);
        }
        buf
    }


    #[test]
    fn test_msg_parsing() {
        let a = Msg{c: b'Q', a: 1000, b: 100000};
        let b = msg_to_bytes(&a);
        let a_parsed = parse_msg(&b);
        assert_eq!(a, a_parsed);
    }

    #[test]
    fn test_sample_sesssion() {
        let mut session = SessionHandler::default();
        assert_eq!(None, session.handle_msg(Msg{c: b'I', a: 12345, b: 101}));
        assert_eq!(None, session.handle_msg(Msg{c: b'I', a: 12346, b: 102}));
        assert_eq!(None, session.handle_msg(Msg{c: b'I', a: 12347, b: 100}));
        assert_eq!(None, session.handle_msg(Msg{c: b'I', a: 40960, b: 5}));
        assert_eq!(Some(101), session.handle_msg(Msg{c: b'Q', a: 12288, b: 16384}));
        assert_eq!(Some(0), session.handle_msg(Msg{c: b'Q', a: 16384, b: 12288}));
        assert_eq!(Some(0), session.handle_msg(Msg{c: b'Q', a: 16384, b: 16384}));
        assert_eq!(Some(0), session.handle_msg(Msg{c: b'Q', a: 16384, b: 16385}));
    }

}
