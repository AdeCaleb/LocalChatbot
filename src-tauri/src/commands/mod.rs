/// Basic chat command - will be expanded with RAG pipeline
#[tauri::command]
pub async fn chat(message: String) -> Result<String, String> {
    // Placeholder response - will integrate RAG + LLM later
    Ok(format!("Echo: {}", message))
}
