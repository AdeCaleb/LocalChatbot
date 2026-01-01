import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { Document, IndexStatus } from '@/types';

/**
 * Backend response type for documents.
 * Matches the DocumentResponse struct in Rust.
 */
interface BackendDocument {
  id: string;
  name: string;
  type: string;
  size: number;
  uploadedAt: string;
}

/**
 * Convert backend document to frontend format.
 */
function convertBackendDocument(doc: BackendDocument): Document {
  return {
    id: doc.id,
    name: doc.name,
    type: doc.type as 'pdf' | 'txt' | 'md',
    size: doc.size,
    uploadedAt: new Date(doc.uploadedAt),
  };
}

/**
 * Custom hook for managing documents with SQLite persistence.
 *
 * This hook handles:
 * - Loading documents from the backend on mount
 * - Opening file dialogs for document selection
 * - Uploading documents (copies to app directory, extracts text)
 * - Deleting documents
 *
 * Key concepts:
 * - Tauri dialog plugin for native file picker
 * - invoke() to call Rust backend commands
 * - IndexStatus tracks whether documents need re-indexing
 */
export function useDocuments() {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [indexStatus, setIndexStatus] = useState<IndexStatus>('ready');
  const [isUploading, setIsUploading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load all documents on mount
  useEffect(() => {
    async function loadDocuments() {
      try {
        setIsInitializing(true);
        setError(null);

        const backendDocs = await invoke<BackendDocument[]>('get_all_documents');
        const frontendDocs = backendDocs.map(convertBackendDocument);
        setDocuments(frontendDocs);
      } catch (err) {
        console.error('Failed to load documents:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsInitializing(false);
      }
    }

    loadDocuments();
  }, []);

  /**
   * Open file dialog and upload selected document.
   *
   * Uses Tauri's dialog plugin to show a native file picker.
   * Selected files are sent to the Rust backend for processing.
   */
  const uploadDocument = useCallback(async () => {
    try {
      setError(null);

      // Open native file dialog
      // The `open` function returns the selected file path(s) or null if cancelled
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Documents',
            extensions: ['pdf', 'txt', 'md', 'markdown'],
          },
        ],
      });

      // User cancelled the dialog
      if (!selected) {
        return null;
      }

      setIsUploading(true);

      // Send the file path to the backend for processing
      // The backend will:
      // 1. Read the file
      // 2. Extract text (especially for PDFs)
      // 3. Copy to app's documents directory
      // 4. Save metadata and content to database
      const backendDoc = await invoke<BackendDocument>('upload_document', {
        filePath: selected,
      });

      const newDoc = convertBackendDocument(backendDoc);
      setDocuments((prev) => [...prev, newDoc]);

      // Mark index as needing rebuild
      // (In a full implementation, this would trigger re-indexing)
      setIndexStatus('indexing');

      // Simulate indexing delay for now
      // TODO: Replace with actual vector embedding/indexing
      setTimeout(() => {
        setIndexStatus('ready');
      }, 1500);

      return newDoc;
    } catch (err) {
      console.error('Failed to upload document:', err);
      setError(err instanceof Error ? err.message : String(err));
      return null;
    } finally {
      setIsUploading(false);
    }
  }, []);

  /**
   * Delete a document from the system.
   *
   * This removes:
   * - The file from the app's documents directory
   * - The metadata from the database
   * - The extracted text content
   */
  const deleteDocument = useCallback(async (documentId: string) => {
    try {
      setError(null);

      // Optimistic update
      setDocuments((prev) => prev.filter((d) => d.id !== documentId));

      // Delete from backend
      await invoke('delete_document_cmd', { documentId });
    } catch (err) {
      console.error('Failed to delete document:', err);
      setError(err instanceof Error ? err.message : String(err));

      // Reload documents on error to restore state
      const backendDocs = await invoke<BackendDocument[]>('get_all_documents');
      setDocuments(backendDocs.map(convertBackendDocument));
    }
  }, []);

  /**
   * Rebuild the vector index for all documents.
   *
   * TODO: This will trigger re-embedding and re-indexing
   * of all documents when the RAG pipeline is implemented.
   */
  const rebuildIndex = useCallback(async () => {
    setIndexStatus('indexing');
    // TODO: Implement actual index rebuild with embeddings
    await new Promise((resolve) => setTimeout(resolve, 2000));
    setIndexStatus('ready');
  }, []);

  return {
    documents,
    indexStatus,
    isUploading,
    isInitializing,
    error,
    uploadDocument,
    deleteDocument,
    rebuildIndex,
  };
}
