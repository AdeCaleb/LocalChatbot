import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Chat,
  Message,
  DocumentSource,
  BackendChat,
  BackendChatWithMessages,
  BackendMessage,
  convertBackendChat,
  convertBackendChatListItem,
  convertBackendMessage,
} from '@/types';

/**
 * Custom hook for managing chat state with SQLite persistence.
 *
 * This hook provides a clean interface for the UI while handling
 * all communication with the Rust backend via Tauri's invoke API.
 *
 * Key concepts demonstrated:
 * - useEffect for data loading on mount
 * - useCallback for stable function references
 * - Error handling with try/catch
 * - Optimistic updates for responsive UI
 */
export function useChat() {
  const [chats, setChats] = useState<Chat[]>([]);
  const [activeChatId, setActiveChatId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Derive active chat from chats array
  const activeChat = chats.find((c) => c.id === activeChatId) || null;

  // Load all chats on mount
  useEffect(() => {
    async function loadChats() {
      try {
        setIsInitializing(true);
        setError(null);

        // Invoke the Rust `get_all_chats` command
        // The type parameter tells TypeScript what we expect back
        const backendChats = await invoke<BackendChat[]>('get_all_chats');

        // Convert backend format to frontend format
        const frontendChats = backendChats.map(convertBackendChatListItem);
        setChats(frontendChats);

        // If there are chats, select the most recent one
        if (frontendChats.length > 0) {
          setActiveChatId(frontendChats[0].id);
        }
      } catch (err) {
        console.error('Failed to load chats:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsInitializing(false);
      }
    }

    loadChats();
  }, []);

  // Load full chat (with messages) when active chat changes
  useEffect(() => {
    async function loadActiveChat() {
      if (!activeChatId) return;

      // Skip if messages are already loaded
      const existing = chats.find((c) => c.id === activeChatId);
      if (existing && existing.messages.length > 0) return;

      try {
        const backendChat = await invoke<BackendChatWithMessages | null>('get_chat', {
          chatId: activeChatId,
        });

        if (backendChat) {
          const fullChat = convertBackendChat(backendChat);

          // Update the chat in state with its messages
          setChats((prev) =>
            prev.map((c) => (c.id === activeChatId ? fullChat : c))
          );
        }
      } catch (err) {
        console.error('Failed to load chat:', err);
      }
    }

    loadActiveChat();
  }, [activeChatId, chats]);

  // Create a new chat
  const createChat = useCallback(async () => {
    try {
      setError(null);

      const backendChat = await invoke<BackendChatWithMessages>('create_chat');
      const newChat = convertBackendChat(backendChat);

      // Add to beginning of list (most recent first)
      setChats((prev) => [newChat, ...prev]);
      setActiveChatId(newChat.id);

      return newChat;
    } catch (err) {
      console.error('Failed to create chat:', err);
      setError(err instanceof Error ? err.message : String(err));
      return null;
    }
  }, []);

  // Delete a chat
  const deleteChat = useCallback(
    async (chatId: string) => {
      try {
        setError(null);

        // Optimistic update - remove immediately for responsive UI
        setChats((prev) => prev.filter((c) => c.id !== chatId));

        // Clear active chat if it's the one being deleted
        if (activeChatId === chatId) {
          setActiveChatId(null);
        }

        // Then persist to backend
        await invoke('delete_chat', { chatId });
      } catch (err) {
        console.error('Failed to delete chat:', err);
        setError(err instanceof Error ? err.message : String(err));

        // Reload chats to restore state on error
        const backendChats = await invoke<BackendChat[]>('get_all_chats');
        setChats(backendChats.map(convertBackendChatListItem));
      }
    },
    [activeChatId]
  );

  // Send a message and get AI response
  const sendMessage = useCallback(
    async (content: string) => {
      try {
        setError(null);
        setIsLoading(true);

        // Auto-create a chat if none exists
        let chatId = activeChatId;
        let isNewChat = false;
        if (!chatId) {
          const backendChat = await invoke<BackendChatWithMessages>('create_chat');
          const newChat = convertBackendChat(backendChat);
          setChats((prev) => [newChat, ...prev]);
          setActiveChatId(newChat.id);
          chatId = newChat.id;
          isNewChat = true;
        }

        // 1. Add user message to backend
        const userMessageResult = await invoke<BackendMessage>('add_message', {
          input: {
            chatId,
            role: 'user',
            content,
            sources: null,
          },
        });

        const userMessage = convertBackendMessage(userMessageResult);

        // 2. Update local state with user message
        // For new chats, this is definitely the first message
        const isFirstMessage =
          isNewChat || chats.find((c) => c.id === chatId)?.messages.length === 0;

        setChats((prev) =>
          prev.map((chat) => {
            if (chat.id === chatId) {
              return {
                ...chat,
                messages: [...chat.messages, userMessage],
                updatedAt: new Date(),
              };
            }
            return chat;
          })
        );

        // 3. Update chat title if this is the first message
        if (isFirstMessage) {
          const newTitle =
            content.slice(0, 30) + (content.length > 30 ? '...' : '');
          await invoke('update_chat_title', {
            chatId,
            title: newTitle,
          });

          setChats((prev) =>
            prev.map((chat) => {
              if (chat.id === chatId) {
                return { ...chat, title: newTitle };
              }
              return chat;
            })
          );
        }

        // 4. Get AI response (placeholder - will use RAG later)
        // For now we use mock sources; later this will come from the RAG pipeline
        const mockSources: DocumentSource[] = [
          {
            documentId: 'd1',
            documentName: 'knowledge-base.pdf',
            chunk: 'Relevant context from your documents...',
            relevance: 0.92,
          },
          {
            documentId: 'd2',
            documentName: 'notes.md',
            chunk: 'Additional supporting information...',
            relevance: 0.85,
          },
        ];

        // Simulate AI thinking time
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const aiResponse = `Based on your documents, here's what I found regarding "${content}":\n\nThis is a mock response. In a real implementation, this would be generated by your AI model using RAG to retrieve relevant context from your indexed documents.`;

        // 5. Add assistant message to backend
        const assistantMessageResult = await invoke<BackendMessage>('add_message', {
          input: {
            chatId,
            role: 'assistant',
            content: aiResponse,
            sources: JSON.stringify(mockSources),
          },
        });

        const assistantMessage = convertBackendMessage(assistantMessageResult);

        // 6. Update local state with assistant message
        setChats((prev) =>
          prev.map((chat) => {
            if (chat.id === chatId) {
              return {
                ...chat,
                messages: [...chat.messages, assistantMessage],
                updatedAt: new Date(),
              };
            }
            return chat;
          })
        );
      } catch (err) {
        console.error('Failed to send message:', err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    },
    [activeChatId, chats]
  );

  return {
    chats,
    activeChat,
    activeChatId,
    setActiveChatId,
    createChat,
    deleteChat,
    sendMessage,
    isLoading,
    isInitializing,
    error,
  };
}
