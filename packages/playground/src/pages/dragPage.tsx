import { useState, useCallback } from 'react';
import { C } from '../theme';

interface Square {
  id: number;
  x: number;
  y: number;
  color: string;
}

const COLORS = [C.accent, C.primary, C.success, C.warning, C.danger];

const INITIAL: Square[] = COLORS.map((color, i) => ({
  id: i,
  x: 30 + i * 96,
  y: 40,
  color,
}));

interface Drag {
  id: number;
  // Window-space distance from the cursor to the square's origin at grab time.
  // Using window coords keeps the math independent of which element the
  // bubbled mousemove targets (localX/localY are relative to that target).
  grabX: number;
  grabY: number;
}

const SIZE = 72;

export function DragPage() {
  const [squares, setSquares] = useState<Square[]>(INITIAL);
  const [drag, setDrag] = useState<Drag | null>(null);

  const onMove = useCallback(
    (x: number, y: number) => {
      if (!drag) return;
      setSquares((prev) =>
        prev.map((s) =>
          s.id === drag.id ? { ...s, x: x - drag.grabX, y: y - drag.grabY } : s,
        ),
      );
    },
    [drag],
  );

  return (
    <view display="flex" flexDir="col" h="full">
      <view
        display="flex"
        flexDir="col"
        gap={8}
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Drag
        </view>
        <view fontSize={12} color={C.textMuted}>
          Press a square and move it. Position is computed from the mouse
          event's localX / localY (relative to the canvas and the square).
        </view>
      </view>

      <view flex={1} p={24}>
        <view
          position="relative"
          w="full"
          h="full"
          bg={C.surface}
          rounded={12}
          border={1}
          borderColor={C.border}
          onMouseMove={(e) => onMove(e.x, e.y)}
          onMouseUp={() => setDrag(null)}
          onMouseLeave={() => setDrag(null)}
        >
          {squares.map((s) => {
            const active = drag?.id === s.id;
            return (
              <view
                key={s.id}
                position="absolute"
                left={s.x}
                top={s.y}
                w={SIZE}
                h={SIZE}
                bg={s.color}
                rounded={10}
                opacity={active ? 0.85 : 1}
                scale={active ? 1.08 : 1}
                border={2}
                borderColor={active ? C.text : 'transparent'}
                display="flex"
                items="center"
                justify="center"
                cursor={active ? 'grabbing' : 'grab'}
                onMouseDown={(e) => {
                  setDrag({ id: s.id, grabX: e.x - s.x, grabY: e.y - s.y });
                }}
              >
                <text fontSize={12} fontWeight={800} color={C.bg}>
                  {`${Math.round(s.x)}, ${Math.round(s.y)}`}
                </text>
              </view>
            );
          })}
        </view>
      </view>
    </view>
  );
}
