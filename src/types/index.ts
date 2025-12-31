export interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  sources?: DocumentSource[];
}

export interface DocumentSource {
  documentId: string;
  documentName: string;
  chunk: string;
  relevance: number;
}

export interface Chat {
  id: string;
  title: string;
  messages: Message[];
  createdAt: Date;
  updatedAt: Date;
}

export interface Document {
  id: string;
  name: string;
  type: 'pdf' | 'txt' | 'md';
  size: number;
  uploadedAt: Date;
}

export interface Settings {
  temperature: number;
  maxTokens: number;
  chunkSize: number;
  topK: number;
  theme: 'light' | 'dark' | 'system';
}

export type IndexStatus = 'ready' | 'indexing' | 'error';

export type ModelStatus = 'ready' | 'loading' | 'error';
