import { useState } from 'react';
import { Clipboard } from 'uzumaki';
import { C } from '../theme';

export function ClipboardPage() {
  const [writeText, setWriteText] = useState('Hello from uzumaki!');
  const [readResult, setReadResult] = useState<string | null>(null);
  const [status, setStatus] = useState<string>('');

  async function handleWrite() {
    setStatus('writing…');
    const ok = await Clipboard.writeText(writeText);
    setStatus(ok ? `wrote ${writeText.length} chars` : 'write failed');
  }

  async function handleRead() {
    setStatus('reading…');
    const text = await Clipboard.readText();
    setReadResult(text);
    setStatus(text == null ? 'clipboard empty' : `read ${text.length} chars`);
  }

  return (
    <view display="flex" flexDir="col" h="full" scrollY scrollbarRadius={5}>
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        gap={8}
        borderBottom="1"
        borderColor={C.border}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Clipboard
        </view>
        <view fontSize={12} color={C.textMuted}>
          Imperative async read/write via the `Clipboard` API
        </view>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <view
          display="flex"
          flexDir="col"
          gap={10}
          p={20}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={13} fontWeight={700} color={C.text}>
            Write
          </text>
          <input
            value={writeText}
            onInput={(e: any) => setWriteText(e.target.value ?? '')}
            placeholder="text to copy"
            px={10}
            py={8}
            rounded={6}
            bg={C.surface3}
            border="1"
            borderColor={C.border}
            color={C.text}
            fontSize={13}
          />
          <button
            onClick={handleWrite}
            px={14}
            py={8}
            rounded={6}
            bg={C.accentDark}
            hover:bg={C.accentDim}
            border={1}
            borderColor={C.accent}
            cursor="pointer"
            display="flex"
            items="center"
            justify="center"
          >
            <text fontSize={13} fontWeight={700} color={C.accentHi}>
              Write Text
            </text>
          </button>
        </view>

        <view
          display="flex"
          flexDir="col"
          gap={10}
          p={20}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={13} fontWeight={700} color={C.text}>
            Read
          </text>
          <button
            onClick={handleRead}
            px={14}
            py={8}
            rounded={6}
            bg={C.surface3}
            hover:bg={C.surface}
            border={1}
            borderColor={C.border}
            cursor="pointer"
            display="flex"
            items="center"
            justify="center"
          >
            <text fontSize={13} fontWeight={700} color={C.text}>
              Read Text
            </text>
          </button>
          <view
            selectable
            p={12}
            rounded={6}
            bg={C.surface3}
            border="1"
            borderColor={C.border}
            minH={48}
          >
            <text
              fontSize={13}
              color={readResult == null ? C.textMuted : C.text}
            >
              {readResult ?? '(empty)'}
            </text>
          </view>
        </view>

        <view fontSize={12} color={C.textMuted} px={4}>
          {status}
        </view>
      </view>
    </view>
  );
}
