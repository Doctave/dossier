use thiserror::Error;

pub type Result<T> = std::result::Result<T, DossierError>;

#[derive(Error, Debug)]
pub enum DossierError {}
