import { useState, useCallback } from 'react';
import { Document, IndexStatus } from '@/types';

// Mock data for demo purposes
const mockDocuments: Document[] = [
  {
    id: 'd1',
    name: 'rag-overview.pdf',
    type: 'pdf',
    size: 2456000,
    uploadedAt: new Date(Date.now() - 86400000),
  },
  {
    id: 'd2',
    name: 'implementation-notes.md',
    type: 'md',
    size: 15600,
    uploadedAt: new Date(Date.now() - 172800000),
  },
  {
    id: 'd3',
    name: 'api-reference.txt',
    type: 'txt',
    size: 8900,
    uploadedAt: new Date(Date.now() - 259200000),
  },
];

// TODO: Replace with actual backend API calls
export function useDocuments() {
  const [documents, setDocuments] = useState<Document[]>(mockDocuments);
  const [indexStatus, setIndexStatus] = useState<IndexStatus>('ready');
  const [isUploading, setIsUploading] = useState(false);

  const uploadDocument = useCallback(async (file: File) => {
    setIsUploading(true);

    // TODO: Replace with actual file upload to backend
    await new Promise(resolve => setTimeout(resolve, 1000));

    const fileType = file.name.split('.').pop()?.toLowerCase() as 'pdf' | 'txt' | 'md';
    
    const newDoc: Document = {
      id: `doc-${Date.now()}`,
      name: file.name,
      type: fileType || 'txt',
      size: file.size,
      uploadedAt: new Date(),
    };

    setDocuments(prev => [...prev, newDoc]);
    setIsUploading(false);

    // Simulate indexing
    setIndexStatus('indexing');
    await new Promise(resolve => setTimeout(resolve, 2000));
    setIndexStatus('ready');

    return newDoc;
  }, []);

  const deleteDocument = useCallback(async (documentId: string) => {
    // TODO: Replace with actual backend delete call
    setDocuments(prev => prev.filter(d => d.id !== documentId));
  }, []);

  const rebuildIndex = useCallback(async () => {
    setIndexStatus('indexing');
    // TODO: Replace with actual index rebuild call
    await new Promise(resolve => setTimeout(resolve, 3000));
    setIndexStatus('ready');
  }, []);

  return {
    documents,
    indexStatus,
    isUploading,
    uploadDocument,
    deleteDocument,
    rebuildIndex,
  };
}
