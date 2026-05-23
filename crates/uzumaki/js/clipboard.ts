import core from 'ext:uzumaki/core.ts';

export const Clipboard = {
  readText(): Promise<string | null> {
    return core.readClipboardText();
  },
  writeText(text: string): Promise<boolean> {
    return core.writeClipboardText(text);
  },
};
