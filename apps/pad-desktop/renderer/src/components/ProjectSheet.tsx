import { useState } from "react";
import { Icon } from "./Icons";
import { ModalSheet } from "./ModalSheet";

function suggestedProjectName(directory: string): string {
  const normalized = directory.replace(/[\\/]+$/, "");
  return normalized.split(/[\\/]/).filter(Boolean).at(-1) ?? "新项目";
}

export function ProjectSheet({
  busy,
  onChooseDirectory,
  onCreate,
  onCancel,
}: {
  busy: boolean;
  onChooseDirectory(): Promise<string | null>;
  onCreate(name: string, directory: string): Promise<void>;
  onCancel(): void;
}) {
  const [name, setName] = useState("");
  const [directory, setDirectory] = useState("");

  async function chooseDirectory() {
    const selected = await onChooseDirectory();
    if (!selected) return;
    setDirectory(selected);
    setName((current) => current.trim() || suggestedProjectName(selected));
  }

  async function submit() {
    if (busy || !directory.trim()) return;
    await onCreate(name.trim() || suggestedProjectName(directory), directory.trim());
  }

  return (
    <ModalSheet
      labelledBy="create-project-title"
      describedBy="create-project-description"
      className="project-create-sheet"
      busy={busy}
      onDismiss={onCancel}
    >
      <div className="auth-sheet-icon"><Icon name="folder" /></div>
      <h2 id="create-project-title">添加项目</h2>
      <p id="create-project-description">选择一个本地文件夹。PAD 只会把它加入当前账号，不会导入 Codex 或 ChatGPT 的项目记录。</p>
      <label className="auth-input">
        <span>项目名称</span>
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="选择目录后自动填写"
        />
      </label>
      <label className="auth-input project-directory-input">
        <span>项目文件夹</span>
        <div className="project-directory-row">
          <input
            value={directory}
            onChange={(event) => setDirectory(event.target.value)}
            placeholder="请选择本地文件夹"
          />
          <button type="button" disabled={busy} onClick={() => void chooseDirectory()}>选择…</button>
        </div>
      </label>
      <div className="auth-sheet-actions">
        <button disabled={busy} onClick={onCancel}>取消</button>
        <button className="is-primary" disabled={busy || !directory.trim()} onClick={() => void submit()}>添加项目</button>
      </div>
    </ModalSheet>
  );
}
