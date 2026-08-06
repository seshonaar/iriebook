use iriebook::resource_access::file;
use iriebook::utilities::types::{
    BookConfig, IdentifierDisplayMode, PdfPageProfile, PdfPageProfileInfo, PdfProfileImageKind,
    available_pdf_page_profiles,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct PdfProfileState {
    pub active_profile: PdfPageProfile,
    pub available_profiles: Vec<PdfPageProfileInfo>,
    pub active_label: String,
    pub width: String,
    pub height: String,
    pub identifier_display: IdentifierDisplayMode,
    pub profile_path: String,
    pub cover_path: Option<String>,
    pub print_cover_path: Option<String>,
}

pub fn get_pdf_profile_state(book_path: &Path) -> Result<PdfProfileState, String> {
    let config = file::load_book_config(book_path)
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    pdf_profile_state_from_config(book_path, &config)
}

pub fn set_active_pdf_profile(
    book_path: &Path,
    profile: PdfPageProfile,
) -> Result<PdfProfileState, String> {
    let config =
        file::set_active_pdf_profile(book_path, profile).map_err(|error| error.to_string())?;
    pdf_profile_state_from_config(book_path, &config)
}

pub fn replace_pdf_profile_image(
    book_path: &Path,
    source_image: &Path,
    kind: PdfProfileImageKind,
) -> Result<PdfProfileState, String> {
    let config = file::load_book_config(book_path)
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let profile = config.pdf.active_profile;
    file::replace_pdf_profile_image(book_path, source_image, profile, kind)
        .map_err(|error| error.to_string())?;
    pdf_profile_state_from_config(book_path, &config)
}

fn pdf_profile_state_from_config(
    book_path: &Path,
    config: &BookConfig,
) -> Result<PdfProfileState, String> {
    let profile = config.pdf.active_profile;
    file::materialize_pdf_profile(book_path, profile).map_err(|error| error.to_string())?;
    let output_profile = profile.output_profile();
    let profile_path = file::get_pdf_profile_file(book_path, profile, "profile.json")
        .map_err(|error| error.to_string())?;
    let cover_path = existing_profile_image_path(book_path, profile, PdfProfileImageKind::Cover)?;
    let print_cover_path =
        existing_profile_image_path(book_path, profile, PdfProfileImageKind::PrintCover)?;

    Ok(PdfProfileState {
        active_profile: profile,
        available_profiles: available_pdf_page_profiles(),
        active_label: output_profile.label.to_string(),
        width: output_profile.page.width.to_string(),
        height: output_profile.page.height.to_string(),
        identifier_display: output_profile.identifier_display,
        profile_path: path_to_string(profile_path),
        cover_path: cover_path.map(path_to_string),
        print_cover_path: print_cover_path.map(path_to_string),
    })
}

fn existing_profile_image_path(
    book_path: &Path,
    profile: PdfPageProfile,
    kind: PdfProfileImageKind,
) -> Result<Option<PathBuf>, String> {
    let path = file::get_pdf_profile_image_file(book_path, profile, kind)
        .map_err(|error| error.to_string())?;
    Ok(path.exists().then_some(path))
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
