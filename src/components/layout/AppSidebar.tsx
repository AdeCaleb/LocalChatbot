import { Plus, ChevronLeft, ChevronRight } from 'lucide-react';
import { Chat, Document, IndexStatus as IndexStatusType } from '@/types';
import { ChatHistory } from '@/components/sidebar/ChatHistory';
import { DocumentList } from '@/components/sidebar/DocumentList';
import { IndexStatus } from '@/components/sidebar/IndexStatus';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface AppSidebarProps {
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  chats: Chat[];
  activeChatId: string | null;
  onSelectChat: (chatId: string) => void;
  onDeleteChat: (chatId: string) => void;
  onNewChat: () => void;
  documents: Document[];
  onDeleteDocument: (documentId: string) => void;
  onUploadDocument: (file: File) => void;
  isUploading: boolean;
  indexStatus: IndexStatusType;
  onRebuildIndex: () => void;
}

export function AppSidebar({
  isCollapsed,
  onToggleCollapse,
  chats,
  activeChatId,
  onSelectChat,
  onDeleteChat,
  onNewChat,
  documents,
  onDeleteDocument,
  onUploadDocument,
  isUploading,
  indexStatus,
  onRebuildIndex,
}: AppSidebarProps) {
  return (
    <aside
      className={cn(
        'flex flex-col bg-sidebar border-r border-sidebar-border transition-all duration-300',
        isCollapsed ? 'w-0 overflow-hidden' : 'w-72'
      )}
    >
      {/* Header with collapse toggle */}
      <div className="flex items-center justify-between p-3 border-b border-sidebar-border">
        <h2 className="font-semibold text-sidebar-foreground">Chats</h2>
        <Button
          variant="ghost"
          size="icon"
          onClick={onToggleCollapse}
          className="h-8 w-8 text-sidebar-foreground"
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
      </div>

      {/* New Chat Button */}
      <div className="p-3">
        <Button onClick={onNewChat} className="w-full gap-2">
          <Plus className="h-4 w-4" />
          New Chat
        </Button>
      </div>

      {/* Chat History */}
      <div className="flex-1 flex flex-col min-h-0">
        <div className="px-3 pb-2">
          <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Conversations
          </h3>
        </div>
        <ChatHistory
          chats={chats}
          activeChatId={activeChatId}
          onSelectChat={onSelectChat}
          onDeleteChat={onDeleteChat}
        />
      </div>

      <Separator className="bg-sidebar-border" />

      {/* Documents Section */}
      <div className="flex flex-col py-3">
        <div className="flex items-center justify-between px-3 pb-2">
          <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            Knowledge Base
          </h3>
          <span className="rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
            {documents.length} docs
          </span>
        </div>
        <DocumentList
          documents={documents}
          onDelete={onDeleteDocument}
          onUpload={onUploadDocument}
          isUploading={isUploading}
        />
      </div>

      {/* Index Status */}
      <IndexStatus status={indexStatus} onRebuild={onRebuildIndex} />
    </aside>
  );
}

// Floating toggle button for collapsed state
export function SidebarToggle({
  isCollapsed,
  onToggle,
}: {
  isCollapsed: boolean;
  onToggle: () => void;
}) {
  if (!isCollapsed) return null;

  return (
    <Button
      variant="outline"
      size="icon"
      onClick={onToggle}
      className="fixed left-4 top-4 z-40 h-9 w-9 bg-card shadow-md"
    >
      <ChevronRight className="h-4 w-4" />
    </Button>
  );
}
