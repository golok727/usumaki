import { useState, useEffect, useMemo } from 'react';
import { C } from '../theme';

export function StressPage() {
  const [fastTick, setFastTick] = useState(0);
  const [clickCount, setClickCount] = useState(0);
  const [tileCount, setTileCount] = useState(80);
  const [tileColors, setTileColors] = useState<string[]>([]);

  const colors12 = useMemo(
    () => [
      '#e2a52e',
      '#d28e20',
      '#c27818',
      '#b56d19',
      '#f97316',
      '#ea580c',
      '#c2410c',
      '#9a3412',
      '#914e17',
      '#783f1b',
      '#67351c',
      '#3c1a0c',
    ],
    [],
  );

  useEffect(() => {
    const id = setInterval(() => {
      setFastTick((t) => t + 1);
      setTileColors((prev) => {
        const next = [...prev];
        for (let i = 0; i < 5; i++) {
          const idx = Math.floor(Math.random() * tileCount);
          next[idx] = colors12[Math.floor(Math.random() * colors12.length)]!;
        }
        return next;
      });
    }, 60);
    return () => clearInterval(id);
  }, [tileCount, colors12]);

  useEffect(() => {
    setTileColors(
      Array.from(
        { length: tileCount },
        (_, i) => colors12[i % colors12.length],
      ) as string[],
    );
  }, [tileCount, colors12]);

  return (
    <view
      display="flex"
      flexDir="col"
      gap={0}
      h="full"
      scrollY
      scrollbarRadius={5}
    >
      <view
        display="flex"
        flexDir="col"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
        gap={8}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Stress Test
        </view>
        <view fontSize={12} color={C.textMuted}>
          High-frequency updates · Large node counts · cursor kinds
        </view>
      </view>

      <view display="flex" flexDir="col" gap={24} p={24}>
        <view display="flex" flexDir="row" gap={12}>
          <view
            flex={1}
            p={20}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            flexDir="col"
            gap={4}
          >
            <view fontSize={12} color={C.textMuted} fontWeight={600}>
              16hz TICKER
            </view>
            <view
              fontSize={52}
              fontWeight={900}
              color={fastTick % 2 === 0 ? C.accentHi : C.primaryHi}
            >
              {fastTick}
            </view>
            <view fontSize={12} color={C.textMuted}>
              {(fastTick / 16).toFixed(2)}s · ~{fastTick * 16} renders
            </view>
          </view>
          <view
            flex={2}
            p={20}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            flexDir="col"
            gap={10}
          >
            <view fontSize={12} color={C.textMuted} fontWeight={600}>
              PULSE BAR
            </view>
            <view w="95%" h={10} bg={C.surface4} rounded={4}>
              <view
                h={10}
                w={`${Math.min(80, Math.max(5, 50 + Math.sin(fastTick * 0.3) * 45))}%`}
                bg={C.accent}
                rounded={4}
              />
            </view>
            <view w="95%" h={10} bg={C.surface4} rounded={4}>
              <view
                h={10}
                w={`${Math.min(80, Math.max(5, 50 + Math.cos(fastTick * 0.2) * 45))}%`}
                bg={C.primary}
                rounded={4}
              />
            </view>
            <view w="95%" h={10} bg={C.surface4} rounded={4}>
              <view
                h={10}
                w={`${Math.min(80, Math.max(5, 50 + Math.sin(fastTick * 0.5 + 1) * 45))}%`}
                bg={C.success}
                rounded={4}
              />
            </view>
            <view w="95%" h={10} bg={C.surface4} rounded={4}>
              <view
                h={10}
                w={`${Math.min(80, Math.max(5, 50 + Math.cos(fastTick * 0.4 + 2) * 45))}%`}
                bg={C.warning}
                rounded={4}
              />
            </view>
          </view>
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" justify="between">
            <view display="flex" flexDir="row" items="center" gap={8}>
              <text fontSize={14} fontWeight={700} color={C.text}>
                Animated tile grid
              </text>
              <text fontSize={12} color={C.textMuted}>
                ({tileCount} tiles · click to count)
              </text>
            </view>
            <view display="flex" flexDir="row" gap={6} items="center">
              <text fontSize={12} color={C.accentHi} fontWeight={700}>
                {clickCount} clicks
              </text>
              <button
                onClick={() => setTileCount((n) => Math.min(200, n + 20))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={12} color={C.success}>
                  +20
                </text>
              </button>
              <button
                onClick={() => setTileCount((n) => Math.max(20, n - 20))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={12} color={C.danger}>
                  −20
                </text>
              </button>
            </view>
          </view>
          <view display="flex" flexDir="col" gap={4} scroll h={300}>
            {Array.from(
              { length: Math.ceil(tileColors.length / 16) },
              (_, rowIdx) => (
                <view key={rowIdx} display="flex" flexDir="row" gap={4}>
                  {tileColors
                    .slice(rowIdx * 16, (rowIdx + 1) * 16)
                    .map((col, i) => (
                      <view
                        key={rowIdx * 16 + i}
                        w={32}
                        h={32}
                        bg={col}
                        rounded={4}
                        cursor="pointer"
                        onClick={() => setClickCount((c) => c + 1)}
                      >
                        <text
                          fontSize={8}
                          fontWeight={800}
                          color="#fff"
                          opacity={0.5}
                        >
                          {rowIdx * 16 + i}
                        </text>
                      </view>
                    ))}
                </view>
              ),
            )}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Many sibling nodes (40 stat rows)
          </text>
          <view
            display="flex"
            flexDir="col"
            bg={C.surface}
            rounded={8}
            border={1}
            borderColor={C.border}
            scroll
            h={200}
          >
            {Array.from({ length: 40 }, (_, i) => (
              <view
                key={i}
                display="flex"
                flexDir="row"
                items="center"
                justify="between"
                px={16}
                py={8}
                borderBottom={1}
                borderColor={C.border}
                hover:bg={C.surface2}
              >
                <view display="flex" flexDir="row" items="center" gap={10}>
                  <view
                    w={8}
                    h={8}
                    bg={tileColors[i % tileColors.length] ?? C.accent}
                    rounded={4}
                  />
                  <text fontSize={12} color={C.textDim}>
                    Process #{String(i + 1).padStart(3, '0')}
                  </text>
                </view>
                <view display="flex" flexDir="row" gap={16} items="center">
                  <text fontSize={11} color={C.textMuted}>
                    {(Math.sin(fastTick * 0.1 + i) * 40 + 50).toFixed(1)}% cpu
                  </text>
                  <view w={60} h={4} bg={C.surface3} rounded={4}>
                    <view
                      h={4}
                      w={`${Math.abs(Math.sin(fastTick * 0.1 + i)) * 100}%`}
                      bg={tileColors[i % tileColors.length] ?? C.accent}
                      rounded={4}
                    />
                  </view>
                </view>
              </view>
            ))}
          </view>
        </view>
      </view>
    </view>
  );
}
