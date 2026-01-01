import { FileText, File, FileCode, Trash2, Upload } from 'lucide-react';
import { Document } from '@/types';
import { cn } from '@/lib/utils';

interface DocumentListProps {
  documents: Document[];
  onDelete: (documentId: string) => void;
  onUpload: () => void; // Now opens a native file dialog
  isUploading: boolean;
}

function getFileIcon(type: Document['type']) {
  switch (type) {
    case 'pdf':
      return FileText;
    case 'md':
      return FileCode;
    default:
      return File;
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function DocumentList({
  documents,
  onDelete,
  onUpload,
  isUploading,
}: DocumentListProps) {
  return (
    <div className="flex flex-col gap-2 px-2">
      {documents.length > 0 && (
        <div className="space-y-1">
          {documents.map((doc) => {
            const Icon = getFileIcon(doc.type);
            return (
              <div
                key={doc.id}
                className="group flex items-center gap-2 rounded-lg px-3 py-2 text-sm text-sidebar-foreground hover:bg-sidebar-accent/50 transition-colors"
              >
                <Icon className="h-4 w-4 shrink-0 opacity-60" />
                <div className="flex-1 min-w-0">
                  <p className="truncate">{doc.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {formatFileSize(doc.size)}
                  </p>
                </div>
                <button
                  onClick={() => onDelete(doc.id)}
                  className="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 rounded transition-all"
                >
                  <Trash2 className="h-3.5 w-3.5 text-destructive" />
                </button>
              </div>
            );
          })}
        </div>
      )}

      {/* Upload button - uses native Tauri file dialog */}
      <button
        onClick={onUpload}
        disabled={isUploading}
        className={cn(
          'flex cursor-pointer flex-col items-center gap-2 rounded-lg border-2 border-dashed border-sidebar-border p-4 text-center transition-colors hover:border-primary hover:bg-sidebar-accent/30',
          isUploading && 'opacity-50 pointer-events-none'
        )}
      >
        <Upload className="h-5 w-5 text-muted-foreground" />
        <div className="text-xs text-muted-foreground">
          {isUploading ? (
            <span>Uploading...</span>
          ) : (
            <>
              <span className="font-medium text-foreground">Click to upload</span>
              {' '}documents
            </>
          )}
        </div>
        <p className="text-[10px] text-muted-foreground">PDF, TXT, MD</p>
      </button>
    </div>
  );
}
