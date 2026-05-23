import { getWindow, Window } from 'uzumaki';
import { createRoot } from 'uzumaki-react';
import { TemplateApp } from './template';
import { C } from './theme';
import { installThemeVars } from './useTheme';

function WindowPreview({
  title,
  detail,
  bg,
}: {
  title: string;
  detail: string;
  bg: string;
}) {
  return (
    <view
      w="full"
      h="full"
      bg={bg}
      display="flex"
      flexDir="col"
      items="center"
      justify="center"
      gap={10}
      p={20}
    >
      <text fontSize={24} fontWeight={800} color="#f4f4f5">
        {title}
      </text>
      <text fontSize={13} color="#d4d4d8">
        {detail}
      </text>
    </view>
  );
}

function getOrCreateWindow(
  label: string,
  title: string,
  detail: string,
  attrs: ConstructorParameters<typeof Window>[1],
  bg = '#18181b',
) {
  const existing = getWindow(label);
  if (existing) {
    existing.visible = true;
    existing.focus();
    return existing;
  }

  const window = new Window(label, {
    width: 440,
    height: 280,
    ...attrs,
    title,
  });
  installThemeVars(window);
  const root = createRoot(window);
  root.render(<WindowPreview title={title} detail={detail} bg={bg} />);
  return window;
}

export function openTransparentPreview() {
  getOrCreateWindow(
    'transparent-preview',
    'Transparent Preview',
    'Created with transparent=true. If your compositor supports it, the background should punch through.',
    {
      transparent: true,
      decorations: true,
      theme: 'dark',
      position: { x: 120, y: 120 },
    },
    'rgba(24, 24, 27, 0.55)',
  );
}

export function openFramelessPreview() {
  getOrCreateWindow(
    'frameless-preview',
    'Frameless Fixed Window',
    'Created with decorations=false, resizable=false, and explicit min/max size constraints.',
    {
      decorations: false,
      resizable: false,
      minWidth: 440,
      minHeight: 280,
      maxWidth: 440,
      maxHeight: 280,
      theme: 'dark',
      position: { x: 180, y: 180 },
    },
  );
}

export function openPositionedThemePreview() {
  getOrCreateWindow(
    'theme-preview',
    'Positioned Theme Preview',
    'Created with explicit position, dark theme, and maximized=false.',
    {
      theme: 'dark',
      maximized: false,
      position: { x: 240, y: 240 },
      resizable: true,
      decorations: true,
    },
  );
}

export function openMinimizedPreview() {
  getOrCreateWindow(
    'minimized-preview',
    'Minimized Preview',
    'Created with minimized=true. Restore it from your taskbar or window overview.',
    {
      minimized: true,
      theme: 'dark',
      position: { x: 360, y: 360 },
    },
  );
}

export function openWindowLevelPreview() {
  getOrCreateWindow(
    'level-preview',
    'Window Level Preview',
    'Created with windowLevel=alwaysOnTop.',
    {
      windowLevel: 'alwaysOnTop',
      theme: 'dark',
      position: { x: 420, y: 160 },
    },
  );
}

export function openContentProtectedPreview() {
  getOrCreateWindow(
    'protected-preview',
    'Content Protected Preview',
    'Created with contentProtected=true. Support depends on the OS and window manager.',
    {
      contentProtected: true,
      theme: 'dark',
      position: { x: 480, y: 220 },
    },
  );
}

export function openDisabledButtonsPreview() {
  getOrCreateWindow(
    'buttons-preview',
    'Disabled Buttons Preview',
    'Created with closable=false and maximizable=false where supported.',
    {
      closable: false,
      minimizable: true,
      maximizable: false,
      theme: 'dark',
      position: { x: 540, y: 280 },
    },
  );
}

export function openActivePreview() {
  getOrCreateWindow(
    'active-preview',
    'Active Preview',
    'Created with active=true as a best-effort focus hint.',
    {
      active: true,
      theme: 'dark',
      position: { x: 600, y: 340 },
    },
  );
}

export function createHiddenPreview() {
  getOrCreateWindow(
    'hidden-preview',
    'Hidden Preview',
    'This window was created hidden and can be shown from the Window Lab page.',
    {
      visible: false,
      theme: 'dark',
      position: { x: 300, y: 300 },
    },
  );
}

export function showHiddenPreview() {
  const hiddenWindow = getWindow('hidden-preview');
  if (hiddenWindow) {
    hiddenWindow.visible = true;
    hiddenWindow.focus();
  }
}

export function openTemplateWindow() {
  const existing = getWindow('template');
  if (existing) {
    existing.visible = true;
    existing.focus();
    return existing;
  }

  const window = new Window('template', {
    width: 800,
    height: 600,
    title: 'Template Preview',
    rootStyles: {
      bg: C.bg,
      color: C.text,
      fontSize: 14,
    },
  });
  installThemeVars(window);
  const root = createRoot(window);
  root.render(<TemplateApp />);
  return window;
}
