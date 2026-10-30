//! Erro único do núcleo nativo.
//!
//! Cada variante carrega a frase exibida na interface. Detalhes técnicos de
//! bibliotecas ficam encapsulados e nunca chegam à WebView: a serialização
//! entrega apenas o texto da mensagem, preservando o contrato `string` com o
//! frontend.

use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(
        "Não foi possível acessar o banco local. Verifique espaço e permissões e tente novamente."
    )]
    Database(#[from] rusqlite::Error),

    #[error("Banco local indisponível. Reinicie o aplicativo.")]
    DatabaseUnavailable,

    #[error("Banco criado por uma versão mais recente do Harnessy. Atualize o aplicativo.")]
    SchemaTooNew,

    #[error("A tarefa foi alterada ou não existe mais. Seu texto foi preservado; reabra a tarefa antes de aplicar as alterações.")]
    StaleTask,

    #[error("{0}")]
    Validation(&'static str),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<AppError> for std::io::Error {
    fn from(error: AppError) -> Self {
        std::io::Error::other(error.to_string())
    }
}
