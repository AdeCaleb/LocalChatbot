import { useState } from 'react';
import { useChat } from '@/hooks/useChat';
import { useDocuments } from '@/hooks/useDocuments';
import { useSettings } from '@/hooks/useSettings';
import { useEmbedding } from '@/hooks/useEmbedding';
import { AppSidebar, SidebarToggle } from './AppSidebar';
import { ChatHeader } from './ChatHeader';
import { MessageList } from '@/components/chat/MessageList';
import { ChatInput } from '@/components/chat/ChatInput';
import { SettingsModal } from '@/components/settings/SettingsModal';

export function MainLayout() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const {
    chats,
    activeChat,
    activeChatId,
    setActiveChatId,
    createChat,
    deleteChat,
    sendMessage,
    isLoading,
    isInitializing,
  } = useChat();

  const {
    documents,
    indexStatus,
    isUploading,
    uploadDocument,
    deleteDocument,
    rebuildIndex,
  } = useDocuments();

  const { settings, updateSettings } = useSettings();

  const {
    modelStatus,
    search,
    indexAllDocuments,
  } = useEmbedding();

  return (
    <div className="flex h-screen w-full bg-background">
      <SidebarToggle
        isCollapsed={sidebarCollapsed}
        onToggle={() => setSidebarCollapsed(false)}
      />

      <AppSidebar
        isCollapsed={sidebarCollapsed}
        onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
        chats={chats}
        activeChatId={activeChatId}
        onSelectChat={setActiveChatId}
        onDeleteChat={deleteChat}
        onNewChat={createChat}
        documents={documents}
        onDeleteDocument={deleteDocument}
        onUploadDocument={uploadDocument}
        isUploading={isUploading}
        indexStatus={indexStatus}
        onRebuildIndex={rebuildIndex}
      />

      <main className="flex flex-1 flex-col min-w-0">
        <ChatHeader
          modelStatus={modelStatus}
          onOpenSettings={() => setSettingsOpen(true)}
        />

        {isInitializing ? (
          <div className="flex flex-1 items-center justify-center">
            <div className="text-muted-foreground">Loading chats...</div>
          </div>
        ) : (
          <>
            <MessageList
              messages={activeChat?.messages || []}
              isLoading={isLoading}
            />
            <ChatInput
              onSend={sendMessage}
              disabled={indexStatus === 'indexing'}
              isLoading={isLoading}
            />
          </>
        )}
      </main>

      <SettingsModal
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        settings={settings}
        onUpdateSettings={updateSettings}
      />
    </div>
  );
}
