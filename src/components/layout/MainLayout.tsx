import { useState } from 'react';
import { FileText } from 'lucide-react';
import { useChat } from '@/hooks/useChat';
import { useDocuments } from '@/hooks/useDocuments';
import { useSettings } from '@/hooks/useSettings';
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

  const hasDocuments = documents.length > 0;

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
          modelStatus="ready"
          onOpenSettings={() => setSettingsOpen(true)}
        />

        {!hasDocuments ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-4 text-muted-foreground p-8">
            <div className="rounded-full bg-muted p-6">
              <FileText className="h-12 w-12" />
            </div>
            <div className="text-center max-w-md">
              <p className="text-lg font-medium text-foreground">
                Upload documents to get started
              </p>
              <p className="text-sm mt-2">
                Add PDF, TXT, or Markdown files to your knowledge base. Once
                indexed, you can ask questions about their contents.
              </p>
            </div>
          </div>
        ) : (
          <>
            <MessageList
              messages={activeChat?.messages || []}
              isLoading={isLoading}
            />
            <ChatInput
              onSend={sendMessage}
              disabled={!hasDocuments || indexStatus === 'indexing'}
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
