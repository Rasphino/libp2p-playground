use floodsub::{Floodsub, FloodsubEvent};
use futures::prelude::*;
use libp2p::{Multiaddr, NetworkBehaviour, PeerId, Transport, core::upgrade, floodsub, identity, Swarm, mdns::{TokioMdns, MdnsEvent}, mplex, noise, swarm::NetworkBehaviourEventProcess, swarm::SwarmBuilder, tcp::TokioTcpConfig};
use slog::{info, o, Drain, Logger};
use once_cell::sync::Lazy;
use tokio::io::{self, AsyncBufReadExt};

static LOGGER: Lazy<Logger> = Lazy::new(|| {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!())
});

#[derive(NetworkBehaviour)]
struct MyBehavior {
    floodsub: Floodsub,
    mdns: TokioMdns
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehavior {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(message ) = event {
            info!(LOGGER, "Received: '{:?}' from {:?}", String::from_utf8_lossy(&message.data), message.source);
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehavior {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    info!(LOGGER, "Local peer id: {:?}", peer_id);

    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let floodsub_topic = floodsub::Topic::new("chat");
    
    let mut swarm = {
        let mdns = TokioMdns::new()?;
        let mut behavior = MyBehavior {
            floodsub: Floodsub::new(peer_id.clone()),
            mdns
        };

        behavior.floodsub.subscribe(floodsub_topic.clone());

        SwarmBuilder::new(transport, behavior, peer_id)
            .executor(Box::new(|fut| { tokio::spawn(fut); }))
            .build()
    };

    if let Some(to_dial) = std::env::args().nth(1) {
        let addr = to_dial.parse::<Multiaddr>()?;
        Swarm::dial_addr(&mut swarm, addr).expect("Failed to dial.");
        info!(LOGGER, "Dialed {:?}", to_dial)
    }

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;

    let mut listening = false;
    loop {
        let to_publish = {
            tokio::select! {
                line = stdin.try_next() => Some((floodsub_topic.clone(), line?.expect("stdin closed"))),
                event = swarm.next() => {
                    info!(LOGGER, "New Event: {:?}", event);
                    None
                }
            }
        };
        if let Some((topic, line)) = to_publish {
            swarm.floodsub.publish(topic, line.as_bytes());
        }
        if !listening {
            for addr in Swarm::listeners(&swarm) {
                println!("Listening on {:?}", addr);
                listening = true;
            }
        }
    }
}
