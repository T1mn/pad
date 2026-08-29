import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProjectSheet } from "./ProjectSheet";

describe("ProjectSheet", () => {
  it("通过 macOS 目录选择结果创建当前账号项目", async () => {
    const onCreate = vi.fn().mockResolvedValue(undefined);
    const onChooseDirectory = vi.fn().mockResolvedValue("/tmp/PAD 示例");
    render(
      <ProjectSheet
        busy={false}
        onChooseDirectory={onChooseDirectory}
        onCreate={onCreate}
        onCancel={vi.fn()}
      />,
    );
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "选择…" }));
    expect(screen.getByLabelText("项目名称")).toHaveValue("PAD 示例");
    expect(screen.getByLabelText("项目文件夹")).toHaveValue("/tmp/PAD 示例");
    await user.click(screen.getByRole("button", { name: "添加项目" }));

    expect(onCreate).toHaveBeenCalledWith("PAD 示例", "/tmp/PAD 示例");
  });

  it("把键盘焦点圈定在 sheet 内并支持 Escape", async () => {
    const onCancel = vi.fn();
    render(
      <ProjectSheet
        busy={false}
        onChooseDirectory={vi.fn().mockResolvedValue(null)}
        onCreate={vi.fn().mockResolvedValue(undefined)}
        onCancel={onCancel}
      />,
    );
    const user = userEvent.setup();

    expect(screen.getByRole("dialog", { name: "添加项目" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
