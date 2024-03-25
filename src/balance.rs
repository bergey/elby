use std::net::SocketAddr;

type Servers = Vec<SocketAddr>;

trait Balancer {
    /// None if no server is healthy / accepting requests
    fn pick_server(&mut self) -> Option<SocketAddr>;
}

/// No health checks, just rotate through the servers in order
struct RoundRobin {
    servers: Servers,
    next: usize,
}

impl RoundRobin {
    pub fn new(servers: Servers) -> Self {
        RoundRobin {
            servers, next: 0
        }
    }
}

impl Balancer for RoundRobin {
    fn pick_server(&mut self) -> Option<SocketAddr> {
        let n = self.servers.len();
        if n == 0 {
            None
        } else {
            let pick = self.servers[self.next % n];
            self.next = (self.next + 1) % n;
            Some(pick)
        }
    }
}
