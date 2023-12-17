use thiserror::Error;

pub type Result<T> = std::result::Result<T, DossierError>;

#[derive(Error, Debug)]
pub enum DossierError {}

pub type MarkdownString = String;

#[derive(Debug, Clone)]
pub struct Page {
    pub title: String,
    pub description: MarkdownString,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub struct Section {
}
