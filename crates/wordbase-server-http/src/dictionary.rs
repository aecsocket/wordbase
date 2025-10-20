use {
    crate::{App, Result, extractor::ExtractProfile},
    axum::{
        Json,
        extract::{DefaultBodyLimit, Multipart, Path, State},
    },
    utoipa_axum::{
        router::{OpenApiRouter, UtoipaMethodRouterExt},
        routes,
    },
    wordbase_types::{Dictionary, DictionaryId},
};

pub fn routes() -> OpenApiRouter<App> {
    OpenApiRouter::new()
        .routes(routes!(get_all))
        .routes(routes!(get))
        .routes(routes!(import).layer(DefaultBodyLimit::disable()))
        .routes(routes!(delete))
        .routes(routes!(enable))
        .routes(routes!(disable))
}

/// Get all imported dictionaries.
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/dictionary",
    responses((status = OK, body = Vec<Dictionary>))
)]
async fn get_all(State(app): State<App>) -> Json<Vec<Dictionary>> {
    let dictionaries = app
        .storage
        .dictionaries()
        .iter()
        .map(|dict| Dictionary {
            id: dict.id,
            meta: dict.meta.clone(),
            position: 1, // TODO
        })
        .collect();
    Json(dictionaries)
}

/// Get a dictionary by its ID.
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/dictionary/{dictionary_id}",
    params(("dictionary_id", description = "Dictionary ID")),
    responses((status = OK, body = Dictionary))
)]
async fn get(
    State(app): State<App>,
    Path((dictionary_id,)): Path<(DictionaryId,)>,
) -> Result<Json<Dictionary>> {
    let dictionary = app.storage.get_dictionary(dictionary_id)?;
    let dictionary = Dictionary {
        id: dictionary.id,
        meta: dictionary.meta.clone(),
        position: 1, // TODO
    };
    Ok(Json(dictionary))
}

/// Import a dictionary from an archive.
#[axum::debug_handler]
#[utoipa::path(put, path = "/dictionary")]
pub async fn import(State(app): State<App>, data: Multipart) -> Result<()> {
    todo!();
}

/// Delete an imported dictionary by its ID.
#[axum::debug_handler]
#[utoipa::path(
    delete,
    path = "/dictionary/{dictionary_id}",
    params(("dictionary_id", description = "Dictionary ID"))
)]
pub async fn delete(
    State(app): State<App>,
    Path((dictionary_id,)): Path<(DictionaryId,)>,
) -> Result<()> {
    app.storage.remove_dictionary(dictionary_id).await?;
    Ok(())
}

/// Enable a dictionary for the given profile, allowing it to provide record
/// lookup results.
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/dictionary/{dictionary_id}/enable",
    params(
        ExtractProfile,
        ("dictionary_id", description = "Dictionary ID"),
    )
)]
pub async fn enable(
    State(app): State<App>,
    ExtractProfile(profile): ExtractProfile,
    Path((dictionary_id,)): Path<(DictionaryId,)>,
) -> Result<()> {
    app.storage
        .enable_dictionary(profile.id, dictionary_id)
        .await?;
    Ok(())
}

/// Disable a dictionary for the given profile, preventing it from providing
/// record lookup results.
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/dictionary/{dictionary_id}/disable",
    params(
        ExtractProfile,
        ("dictionary_id", description = "Dictionary ID"),
    )
)]
pub async fn disable(
    State(app): State<App>,
    ExtractProfile(profile): ExtractProfile,
    Path((dictionary_id,)): Path<(DictionaryId,)>,
) -> Result<()> {
    app.storage
        .disable_dictionary(profile.id, dictionary_id)
        .await?;
    Ok(())
}

// pub async fn import(app: &App, req: Import) -> EventStream<BoxStream<'static,
// ImportEvent>> {     let import = req
//         .archive
//         .into_vec()
//         .await
//         .map(|archive| {
//             let archive = Bytes::from(archive);
//             app.engine.import_dictionary(move || {
//                 let archive = Cursor::new(archive.clone());
//                 async move { anyhow::Ok(Box::new(archive) as Box<dyn
// Archive>) }             })
//         })
//         .context("failed to read into memory");
//     let stream = async_stream::stream! {        let import = match import {
//             Ok(import) => import,
//             Err(err) => {
//                 yield ImportEvent::Err(ImportErr {
//                     error: format!("{err:?}"),
//                 });
//                 return;
//             }
//         };
//         yield ImportEvent::ReadIntoMemory(ReadIntoMemory {});

//         for await event in import {
//             yield match event {
//                 Ok(import::ImportEvent::DeterminedKind(kind)) => {
//                     ImportEvent::DeterminedKind(DeterminedKind { kind })
//                 }
//                 Ok(import::ImportEvent::ParsedMeta(meta)) => {
//                     ImportEvent::ParsedMeta(meta)
//                 }
//                 Ok(import::ImportEvent::Progress(progress)) => {
//                     ImportEvent::Progress(progress)
//                 }
//                 Ok(import::ImportEvent::Done(dictionary_id)) => {
//                     ImportEvent::Done(ImportDone { dictionary_id })
//                 }
//                 Err(err) => {
//                     ImportEvent::Err(ImportErr { error: format!("{err:?}") })
//                 }
//             }
//         }
//     };
//     EventStream::new(stream.boxed())
// }

// #[derive(Debug, Multipart)]
// pub struct Import {
//     pub archive: Upload,
// }

// #[derive(Debug, Clone, Union)]
// #[oai(discriminator_name = "event_kind")]
// pub enum ImportEvent {
//     ReadIntoMemory(ReadIntoMemory),
//     DeterminedKind(DeterminedKind),
//     ParsedMeta(DictionaryMeta),
//     Progress(ImportProgress),
//     Done(ImportDone),
//     Err(ImportErr),
// }

// #[derive(Debug, Clone, Object)]
// pub struct ReadIntoMemory {}

// #[derive(Debug, Clone, Object)]
// pub struct DeterminedKind {
//     pub kind: DictionaryKind,
// }

// #[derive(Debug, Clone, Object)]
// pub struct ImportDone {
//     pub dictionary_id: DictionaryId,
// }

// #[derive(Debug, Clone, Object)]
// pub struct ImportErr {
//     pub error: String,
// }

// pub async fn position_swap(app: &App, req: PositionSwap) -> Result<()> {
//     app.engine
//         .swap_dictionary_positions(req.a_id, req.b_id)
//         .await?;
//     Ok(())
// }

// #[derive(Debug, Clone, Object)]
// pub struct PositionSwap {
//     pub a_id: DictionaryId,
//     pub b_id: DictionaryId,
// }

// pub async fn enable(app: &App, req: ToggleEnable) -> Result<()> {
//     app.engine
//         .enable_dictionary(req.profile_id, req.dictionary_id)
//         .await?;
//     Ok(())
// }

// pub async fn disable(app: &App, req: ToggleEnable) -> Result<()> {
//     app.engine
//         .disable_dictionary(req.profile_id, req.dictionary_id)
//         .await?;
//     Ok(())
// }

// #[derive(Debug, Clone, Object)]
// pub struct ToggleEnable {
//     pub dictionary_id: DictionaryId,
//     pub profile_id: ProfileId,
// }
