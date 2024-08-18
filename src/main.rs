use std::sync::Arc;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use dashmap::DashMap;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};

struct AppData {
    lock_duration: Duration,
    transfer_locks: DashMap<String, Arc<Mutex<i32>>>,
}

impl AppData {
    fn get_transfer_lock(&self, user_id: &String) -> Arc<Mutex<i32>> {
        let transfer_ref = self.transfer_locks.get(user_id);

        if transfer_ref.is_none() {
            log::info!("Creating new transfer lock for user: {}", user_id);
            self.transfer_locks
                .insert(user_id.to_string(), Arc::new(Mutex::new(0)));
        }

        let transfer_ref = self.transfer_locks.get(user_id).unwrap();
        let arc = Arc::clone(transfer_ref.value());

        log::info!(
            "Memory address lock in get_transfer_lock: {:p}",
            arc.as_ref()
        );

        arc
    }
}

async fn transfer(info: web::Path<String>, data: web::Data<AppData>) -> impl Responder {
    let user_id = info.into_inner();
    let start_time = Utc::now().time();

    log::info!("User: {} transfer...", user_id);

    let guard = data.get_transfer_lock(&user_id);

    log::info!("Memory address lock in transfer: {:p}", guard.as_ref());

    // Lock the process
    let _lock = guard.lock().await;

    // Simulate a long running operation
    sleep(data.lock_duration).await;

    let end_time = Utc::now().time();
    let diff = end_time - start_time;
    let diff = diff.num_milliseconds();
    log::info!("User: {} took {} ms", user_id, diff);

    HttpResponse::Ok().body("Transfer completed")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let app_data = web::Data::new(AppData {
        lock_duration: Duration::from_millis(500),
        transfer_locks: DashMap::new(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .route("users/{user_id}/transfer", web::get().to(transfer))
    })
    .workers(1)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
