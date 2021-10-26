use rocket::form::{FromForm};
use rocket::State;
use rocket::http::Status;
use failsafe::{Config, CircuitBreaker, Error, StateMachine, backoff::{self,Exponential}, failure_policy::{self,ConsecutiveFailures}};
use std::time::Duration;

#[macro_use] extern crate rocket;

#[derive(Debug, FromForm)]
struct Query {
    name: String,
}

struct RocketState {
    circuit : StateMachine<ConsecutiveFailures<Exponential>, ()>
}

fn hello(name: String) -> Result<String,()> {
    if name == "error" {
        return Err(())
    }
    Ok(name)
}

#[get("/hello?<query..>")]
fn api_hello(state: &State<RocketState>, query: Query) -> Result<String, Status> {
    match state.circuit.call(|| hello(query.name)) {
        Err(Error::Inner(_)) => {
            eprintln!("fail");
            return Err(Status::InternalServerError)
        },
        Err(Error::Rejected) => {
            eprintln!("rejected");
            return Err(Status::ServiceUnavailable)
        },
        Ok(x) => {
            return Ok(x)
        }
    }
}

#[launch]
fn rocket() -> _ {
    let back_off = backoff::exponential(Duration::from_secs(10), Duration::from_secs(30));
    let policy = failure_policy::consecutive_failures(3, back_off);
    let circuit_breaker = Config::new()
        .failure_policy(policy)
        .build();
    let hystrix_conf = RocketState{circuit: circuit_breaker};
    rocket::build()
        .manage(hystrix_conf)
        .mount("/",routes![api_hello])
}
