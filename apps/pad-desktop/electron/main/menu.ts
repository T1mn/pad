import { BrowserWindow, Menu, type MenuItemConstructorOptions } from 'electron';
import { DESKTOP_IPC, type DesktopMenuAction } from '../../shared/protocol';

export function installApplicationMenu(): void {
  const send = (action: DesktopMenuAction) => {
    const window = BrowserWindow.getAllWindows().find((candidate) => !candidate.isDestroyed());
    window?.webContents.send(DESKTOP_IPC.event, { type: 'menu_action', action });
  };
  const template: MenuItemConstructorOptions[] = [
    {
      label: 'PAD Desktop',
      submenu: [
        { label: '关于 PAD Desktop', role: 'about' },
        { type: 'separator' },
        { label: '设置…', accelerator: 'CommandOrControl+,', click: () => send('settings') },
        { type: 'separator' },
        { label: '服务', role: 'services', submenu: [] },
        { type: 'separator' },
        { label: '隐藏 PAD Desktop', role: 'hide' },
        { label: '隐藏其他', role: 'hideOthers' },
        { label: '全部显示', role: 'unhide' },
        { type: 'separator' },
        { label: '退出 PAD Desktop', role: 'quit' },
      ],
    },
    {
      label: '文件',
      submenu: [
        { label: '新建任务', accelerator: 'CommandOrControl+N', click: () => send('new_task') },
        { label: '搜索任务', accelerator: 'CommandOrControl+K', click: () => send('search') },
        { type: 'separator' },
        { label: '关闭当前面板', accelerator: 'CommandOrControl+W', click: () => send('close_active') },
      ],
    },
    {
      label: '编辑',
      submenu: [
        { label: '撤销', role: 'undo' },
        { label: '重做', role: 'redo' },
        { type: 'separator' },
        { label: '剪切', role: 'cut' },
        { label: '复制', role: 'copy' },
        { label: '粘贴', role: 'paste' },
        { label: '粘贴并匹配样式', role: 'pasteAndMatchStyle' },
        { label: '删除', role: 'delete' },
        { label: '全选', role: 'selectAll' },
      ],
    },
    {
      label: '显示',
      submenu: [
        { label: '切换侧边栏', accelerator: 'CommandOrControl+B', click: () => send('toggle_sidebar') },
        { label: '切换终端', accelerator: 'CommandOrControl+J', click: () => send('toggle_terminal') },
        { type: 'separator' },
        { label: '进入全屏幕', role: 'togglefullscreen' },
      ],
    },
    {
      label: '窗口',
      submenu: [
        { label: '最小化', role: 'minimize' },
        { label: '缩放', role: 'zoom' },
        { type: 'separator' },
        { label: '前置全部窗口', role: 'front' },
      ],
    },
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}
