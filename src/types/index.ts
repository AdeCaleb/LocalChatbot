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

export type ModelStatus = 'not_loaded' | 'loading' | 'ready' | 'error';

// Types matching Rust backend responses (dates as ISO strings)
export interface BackendMessage {
  id: string;
  chat_id: string;
  role: string;
  content: string;
  timestamp: string; // ISO 8601 string from Rust
  sources: string | null; // JSON string of DocumentSource[]
}

export interface BackendChat {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface BackendChatWithMessages {
  id: string;
  title: string;
  messages: BackendMessage[];
  created_at: string;
  updated_at: string;
}

// Helper to convert backend response to frontend types
export function convertBackendChat(backend: BackendChatWithMessages): Chat {
  return {
    id: backend.id,
    title: backend.title,
    createdAt: new Date(backend.created_at),
    updatedAt: new Date(backend.updated_at),
    messages: backend.messages.map(convertBackendMessage),
  };
}

export function convertBackendMessage(backend: BackendMessage): Message {
  let sources: DocumentSource[] | undefined;
  if (backend.sources) {
    try {
      sources = JSON.parse(backend.sources);
    } catch {
      sources = undefined;
    }
  }

  return {
    id: backend.id,
    role: backend.role as 'user' | 'assistant',
    content: backend.content,
    timestamp: new Date(backend.timestamp),
    sources,
  };
}

// Convert chat list item (without messages)
export function convertBackendChatListItem(backend: BackendChat): Chat {
  return {
    id: backend.id,
    title: backend.title,
    createdAt: new Date(backend.created_at),
    updatedAt: new Date(backend.updated_at),
    messages: [], // Messages not loaded in list view
  };
}
