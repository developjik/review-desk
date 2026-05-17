import { FileCode2 } from "lucide-react";
import { Badge } from "./ui";
import { t } from "../lib/i18n";
import type { ChangedFileContext, Locale } from "../lib/view-models";

export function ChangedFileList({
  locale,
  files,
  selectedFile,
  viewedFiles,
  onSelectFile,
}: {
  locale: Locale;
  files: ChangedFileContext[];
  selectedFile: ChangedFileContext | null;
  viewedFiles: Set<string>;
  onSelectFile: (file: ChangedFileContext) => void;
}) {
  return (
    <div className="min-h-0 border-r border-zinc-800">
      <div className="flex h-11 items-center gap-2 border-b border-zinc-800 px-3 text-sm font-medium">
        <FileCode2 className="h-4 w-4 text-zinc-500" />
        {t(locale, "workspace.changedFiles")}
      </div>
      <div className="h-full overflow-auto pb-12">
        {files.map((file) => {
          const active = selectedFile?.path === file.path;
          const viewed = viewedFiles.has(file.path);
          return (
            <button key={file.path} onClick={() => onSelectFile(file)} className={`grid w-full gap-1 border-b border-zinc-900 px-3 py-2 text-left text-sm hover:bg-zinc-900 ${active ? "bg-zinc-900 text-white" : "text-zinc-300"}`}>
              <div className="flex min-w-0 items-center justify-between gap-2">
                <span className={`truncate ${viewed ? "text-zinc-500" : ""}`}>{file.path}</span>
                <Badge tone={file.patch ? "blue" : "neutral"} className="h-5 text-[10px]">
                  {file.status}
                </Badge>
              </div>
              <div className="flex items-center justify-between text-[11px] text-zinc-500">
                <span>+{file.additions} -{file.deletions}</span>
                {viewed && <span>{t(locale, "workspace.fileViewed")}</span>}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
