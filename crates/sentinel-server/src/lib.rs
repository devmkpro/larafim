//! Servidor local do Sentinel (fase 3): deixa projetos online em
//! `http://projeto.offline`, cada um com sua versão do PHP.

pub mod cgi;
pub mod devserver;
pub mod fcgi;
pub mod gateway;
pub mod hosts;
pub mod job;
pub mod pages;
pub mod pool;
pub mod port;
pub mod proxy;
pub mod resolve;

pub use devserver::{DevEvent, DevServers, DevSpec, DevState, DevStatus, PortCell};
pub use gateway::{Gateway, Routes, SharedRoutes, Site};
pub use pool::{PhpRuntime, PoolSettings, Pools};
