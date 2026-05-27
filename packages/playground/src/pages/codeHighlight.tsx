import { memo, useMemo, useState } from 'react';
import { useWindow } from 'uzumaki-react';
import { C } from '../theme';
import { useTheme } from '../useTheme';
import { highlightTsx } from '../utils/highlighter';

const INITIAL_CODE = `
import { Window } from "uzumaki"
import { createRoot } from "uzumaki-react"

const window = new Window("main", { width: 800, height: 600 });
const root = createRoot(window);
root.render(<view><text>Uzumaki</text></view>)`.trim();

interface TokenRenderer {
  content: string;
  color?: string;
}

const TokenRenderer = memo(function TokenComponent({
  token,
}: {
  token: TokenRenderer;
}) {
  return <text color={token.color}>{token.content}</text>;
});

const LineRenderer = memo(function LineComponent({
  tokens,
  lineNumber,
}: {
  tokens: TokenRenderer[];
  lineNumber: number;
}) {
  return (
    <view display="flex" flexDir="row" w="full" gap={12}>
      <text
        selectable={false}
        color={C.textMuted}
        fontSize={14}
        w={32}
        flexShrink={0}
        textAlign="right"
      >
        {String(lineNumber)}
      </text>
      <view w="full" minW={0} fontSize={16}>
        {tokens.map((token, j) => (
          <TokenRenderer key={j} token={token} />
        ))}
      </view>
    </view>
  );
});

export function ShikiPage() {
  const [code, setCode] = useState(INITIAL_CODE);
  const window = useWindow();
  const theme = useTheme(window);

  const lineTokens = useMemo(() => highlightTsx(code, theme), [code, theme]);

  return (
    <view
      display="flex"
      flexDir="row"
      gap={16}
      p={16}
      w="full"
      h="full"
      bg={C.bg}
    >
      <Panel title="Preview">
        <view
          selectable
          p={16}
          flex={1}
          flexDir="col"
          overflow="auto"
          fontFamily="Geist Mono, monospace"
        >
          {lineTokens.map((tokens, i) => (
            <LineRenderer key={i} tokens={tokens} lineNumber={i + 1} />
          ))}
        </view>
      </Panel>

      <Panel title="Editor">
        <input
          multiline
          value={code}
          onValueChange={setCode}
          flex={1}
          p={16}
          color={C.text}
          fontSize={14}
          bg="transparent"
        />
      </Panel>
    </view>
  );
}

function Panel({ title, children }: { title: string; children: any }) {
  return (
    <view
      display="flex"
      flexDir="col"
      flex={1}
      minW={0}
      bg={C.surface}
      rounded={10}
      border={1}
      borderColor={C.border}
    >
      <view p={12} px={16} borderBottom={1} borderColor={C.border}>
        <text color={C.textSub} fontSize={13} fontWeight="medium">
          {title}
        </text>
      </view>
      {children}
    </view>
  );
}
