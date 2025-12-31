import { MessageSquare, Trash2 } from 'lucide-react';
import { Chat } from '@/types';
import { cn } from '@/lib/utils';

interface ChatHistoryProps {
  chats: Chat[];
  activeChatId: string | null;
  onSelectChat: (chatId: string) => void;
  onDeleteChat: (chatId: string) => void;
}

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return 'Today';
  if (diffDays === 1) return 'Yesterday';
  if (diffDays < 7) return `${diffDays} days ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
  return `${Math.floor(diffDays / 30)} months ago`;
}

export function ChatHistory({
  chats,
  activeChatId,
  onSelectChat,
  onDeleteChat,
}: ChatHistoryProps) {
  if (chats.length === 0) {
    return (
      <div className="px-3 py-6 text-center">
        <div className="mx-auto mb-2 flex h-10 w-10 items-center justify-center rounded-full bg-muted">
          <MessageSquare className="h-5 w-5 text-muted-foreground" />
        </div>
        <p className="text-sm text-muted-foreground">No conversations yet</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-2">
      <div className="space-y-1">
        {chats.map((chat) => (
          <button
            key={chat.id}
            onClick={() => onSelectChat(chat.id)}
            className={cn(
              'group flex w-full items-center gap-2 rounded-lg px-3 py-2.5 text-left text-sm transition-colors',
              activeChatId === chat.id
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            )}
          >
            <MessageSquare className="h-4 w-4 shrink-0 opacity-60" />
            <div className="flex-1 min-w-0">
              <p className="truncate font-medium">{chat.title}</p>
              <p className="text-xs text-muted-foreground">
                {formatRelativeTime(chat.updatedAt)}
              </p>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDeleteChat(chat.id);
              }}
              className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded transition-all"
            >
              <Trash2 className="h-3.5 w-3.5 text-destructive" />
            </button>
          </button>
        ))}
      </div>
    </div>
  );
}
