import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface SearchResult {
  chunk_id: string;
  document_id: string;
  content: string;
  score: number;
}

export interface EmbeddingStats {
  totalEmbeddings: number;
  totalDocuments: number;
}

type ModelStatus = 'not_loaded' | 'loading' | 'ready' | 'error';

export function useEmbedding() {
  const [modelStatus, setModelStatus] = useState<ModelStatus>('not_loaded');
  const [error, setError] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // Initialize the embedding model on mount
  useEffect(() => {
    const initModel = async () => {
      setModelStatus('loading');
      setError(null);

      try {
        // Check if model is already loaded
        const isLoaded = await invoke<boolean>('is_model_loaded');
        if (isLoaded) {
          setModelStatus('ready');
          return;
        }

        // Load the model (downloads ~90MB on first run)
        console.log('Loading embedding model...');
        await invoke<string>('init_embedding_model');
        setModelStatus('ready');
        console.log('Embedding model ready');
      } catch (err) {
        console.error('Failed to load embedding model:', err);
        setError(err instanceof Error ? err.message : String(err));
        setModelStatus('error');
      }
    };

    initModel();
  }, []);

  // Search documents
  const search = useCallback(async (query: string, topK: number = 5): Promise<SearchResult[]> => {
    if (modelStatus !== 'ready') {
      console.warn('Cannot search: model not ready');
      return [];
    }

    if (!query.trim()) {
      setSearchResults([]);
      return [];
    }

    setIsSearching(true);
    try {
      const results = await invoke<SearchResult[]>('search_documents', {
        query,
        topK,
      });
      setSearchResults(results);
      return results;
    } catch (err) {
      console.error('Search failed:', err);
      return [];
    } finally {
      setIsSearching(false);
    }
  }, [modelStatus]);

  // Clear search results
  const clearSearch = useCallback(() => {
    setSearchResults([]);
  }, []);

  // Index all documents (for existing documents)
  const indexAllDocuments = useCallback(async (): Promise<{ docs: number; chunks: number }> => {
    if (modelStatus !== 'ready') {
      throw new Error('Model not ready');
    }

    const [docs, chunks] = await invoke<[number, number]>('index_all_documents');
    return { docs, chunks };
  }, [modelStatus]);

  // Get embedding stats
  const getStats = useCallback(async (): Promise<EmbeddingStats> => {
    const [totalEmbeddings, totalDocuments] = await invoke<[number, number]>('get_embedding_stats');
    return { totalEmbeddings, totalDocuments };
  }, []);

  // Retry loading the model
  const retryInit = useCallback(async () => {
    setModelStatus('loading');
    setError(null);

    try {
      await invoke<string>('init_embedding_model');
      setModelStatus('ready');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setModelStatus('error');
    }
  }, []);

  return {
    modelStatus,
    error,
    searchResults,
    isSearching,
    search,
    clearSearch,
    indexAllDocuments,
    getStats,
    retryInit,
  };
}
