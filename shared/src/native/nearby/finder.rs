use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::spawn;
use tokio::task::spawn_blocking;

const SERVICE_TYPE: &str = "_bitbridge._udp.local.";

struct NearbyFinder {
    daemon: ServiceDaemon,
    public_port: u16
}

impl NearbyFinder {
    pub fn new(public_port: u16) -> Self {
        Self {
            daemon: ServiceDaemon::new().expect("Failed to create mDNS service"),
            public_port
        }
    }

    pub async fn start(&self) {
        let daemon = self.daemon.clone();
        let receiver = spawn_blocking(move || daemon.browse(SERVICE_TYPE).expect("Failed to browse"))
            .await
            .expect("Failed to spawn blocking");

        spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        println!("Resolved a new service: {}", info.get_fullname());
                    }
                    other_event => {
                        println!("Received other event: {:?}", &other_event);
                    }
                }
            }
        });

        let daemon = self.daemon.clone();
        let service_info = self.get_service_info();
        spawn_blocking(move || {
            daemon.register(service_info).expect("Failed to register service");
        })
        .await
        .expect("Failed to spawn blocking for register service");
    }

    fn get_service_info(&self) -> ServiceInfo {
        let instance_name = "bitbridge";
        let properties = [
            ("property_1", "test"),
            ("property_2", "1234")
        ];

        let service = ServiceInfo::new(SERVICE_TYPE, instance_name, "", "", self.public_port, &properties[..])
            .expect("Failed to create service info");

        service.enable_addr_auto()
    }

    pub fn stop(&self) {
        let daemon = self.daemon.clone();
        spawn_blocking(move || {
            let _ = daemon.shutdown();
        });
    }
}
