use dashmap::DashMap;

type FileId = u32;

#[salsa::input(debug)]
pub struct FileText {
    #[returns(ref)]
    pub text: String,
    pub id: FileId,
}


#[derive(Debug, Default)]
pub struct Files {
    files: DashMap<FileId, FileText>,
}
