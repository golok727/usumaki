import { useState } from 'react';
import { C } from '../theme';
import { Divider } from '../components';

type TextAlignValue = 'left' | 'center' | 'right' | 'start' | 'end' | 'justify';

const ALIGN_VALUES: TextAlignValue[] = [
  'left',
  'center',
  'right',
  'start',
  'end',
  'justify',
];

function TextAlignDemo() {
  const [align, setAlign] = useState<TextAlignValue>('start');
  const [parentDisplay, setParentDisplay] = useState<'block' | 'flex'>('block');
  const [singleLine, setSingleLine] = useState('');
  const [multiLine, setMultiLine] = useState('');

  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <text fontSize={14} fontWeight={700} color={C.text}>
          textAlign
        </text>
        <text fontSize={12} color={C.textMuted}>
          Click a value to change alignment
        </text>
      </view>

      <view display="flex" flexDir="row" gap={6} flexWrap="wrap">
        {ALIGN_VALUES.map((v) => (
          <button
            key={v}
            onClick={() => setAlign(v)}
            px={12}
            py={6}
            bg={align === v ? C.accentDim : C.surface3}
            hover:bg={align === v ? C.accent : C.surface4}
            active:bg={C.accentDim}
            rounded={6}
            border={1}
            borderColor={align === v ? C.accent : C.border}
            cursor="pointer"
          >
            <text
              fontSize={12}
              fontWeight={align === v ? 700 : 400}
              color={align === v ? C.accentHi : C.textMuted}
            >
              {v}
            </text>
          </button>
        ))}
      </view>

      <view display="flex" flexDir="row" gap={6}>
        {(['block', 'flex'] as const).map((v) => (
          <button
            key={v}
            onClick={() => setParentDisplay(v)}
            px={12}
            py={6}
            bg={parentDisplay === v ? C.accentDim : C.surface3}
            hover:bg={parentDisplay === v ? C.accent : C.surface4}
            active:bg={C.accentDim}
            rounded={6}
            border={1}
            borderColor={parentDisplay === v ? C.accent : C.border}
            cursor="pointer"
          >
            <text
              fontSize={12}
              fontWeight={parentDisplay === v ? 700 : 400}
              color={parentDisplay === v ? C.accentHi : C.textMuted}
            >
              {v}
            </text>
          </button>
        ))}
      </view>

      <view
        display={parentDisplay}
        flexDir="col"
        items="start"
        p={16}
        bg={C.surface2}
        rounded={8}
        border={1}
        borderColor={C.border}
        selectable
        fontSize={14}
        color={C.text}
        textAlign={align}
      >
        The quick brown{' '}
        <text
          bg={C.accent}
          border={1}
          borderColor={C.accentHi}
          textAlign="center"
          p={6}
          rounded={5}
          fontWeight={700}
          fontSize={16}
        >
          fox
        </text>{' '}
        jumps over the lazy dog. Pack my box with five dozen liquor jugs.
      </view>

      <view display="flex" flexDir="col" gap={8}>
        <view
          display={parentDisplay}
          flexDir="col"
          p={14}
          bg={C.surface2}
          gap={4}
          rounded={8}
          border={1}
          selectable
          borderColor={C.border}
          fontSize={14}
          color={C.text}
          textAlign={align}
        >
          Inline chip before{' '}
          <text
            bg={C.accentDark}
            border={1}
            borderColor={C.accent}
            color={C.accentHi}
            p={4}
            px={8}
            rounded={4}
            fontWeight={700}
          >
            middle
          </text>{' '}
          and after text should keep clear spacing.
        </view>

        <view
          display={parentDisplay}
          flexDir="col"
          items="start"
          p={14}
          bg={C.surface2}
          rounded={8}
          border={1}
          selectable
          borderColor={C.border}
          fontSize={14}
          color={C.text}
          textAlign={align}
          w={360}
        >
          Wrapping inline chips should stay attached to their text while this{' '}
          <text
            bg={C.successDark}
            border={1}
            borderColor={C.success}
            color={C.successHi}
            p={5}
            px={9}
            rounded={5}
            fontWeight={700}
          >
            highlighted phrase
          </text>{' '}
          moves across lines.
        </view>

        <view
          display={parentDisplay}
          flexDir="col"
          p={14}
          bg={C.surface2}
          rounded={8}
          selectable
          border={1}
          borderColor={C.border}
          fontSize={14}
          color={C.text}
          textAlign={align}
        >
          Multiple chips:{' '}
          <text
            bg={C.warningDark}
            border={1}
            borderColor={C.warning}
            color={C.warningHi}
            p={3}
            px={7}
            rounded={4}
            fontWeight={700}
          >
            alpha
          </text>{' '}
          <text
            bg={C.surface4}
            border={1}
            borderColor={C.borderHi}
            color={C.text}
            p={5}
            px={9}
            rounded={5}
            fontWeight={700}
          >
            beta
          </text>{' '}
          <text
            bg={C.accent}
            border={1}
            borderColor={C.accentHi}
            color={C.bg}
            p={6}
            px={10}
            rounded={5}
            fontWeight={700}
          >
            gamma
          </text>
        </view>
      </view>

      <view display="flex" flexDir="col" gap={8}>
        <input
          value={singleLine}
          onValueChange={setSingleLine}
          textAlign={align}
          placeholder="single-line input"
          fontSize={14}
          color={C.text}
          bg={C.surface2}
          p={8}
          rounded={8}
          border={1}
          borderColor={C.border}
          w="full"
        />
        <input
          multiline
          value={multiLine}
          onValueChange={setMultiLine}
          textAlign={align}
          placeholder="multiline input"
          fontSize={14}
          color={C.text}
          bg={C.surface2}
          p={8}
          rounded={8}
          border={1}
          borderColor={C.border}
          w="full"
          h={90}
        />
      </view>
    </view>
  );
}

function AbsClickCounter({ color }: { color: string }) {
  const [count, setCount] = useState(0);
  return (
    <button
      onClick={() => setCount((c) => c + 1)}
      p={7}
      bg={color}
      hover:bg={C.surface4}
      active:bg={C.accentDim}
      rounded={4}
      cursor="pointer"
      display="flex"
      items="center"
      gap={6}
    >
      <text fontSize={11} fontWeight={600} color={C.text}>
        {count}
      </text>
    </button>
  );
}

function AbsolutePositioningDemo() {
  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <view fontSize={14} fontWeight={700} color={C.text}>
          Absolute positioning
        </view>
        <view fontSize={12} color={C.textMuted}>
          position="absolute" with top / right / bottom / left insets
        </view>
      </view>

      <view display="flex" flexDir="row" gap={12}>
        {(
          [
            { label: 'top + left', pos: { top: 8, left: 8 }, c: C.accent },
            { label: 'top + right', pos: { top: 8, right: 8 }, c: C.primary },
            {
              label: 'bottom + right',
              pos: { bottom: 8, right: 8 },
              c: C.success,
            },
            {
              label: 'bottom + left',
              pos: { bottom: 8, left: 8 },
              c: C.warning,
            },
          ] as const
        ).map(({ label, pos, c }) => (
          <view
            key={label}
            flex={1}
            minW={0}
            h={120}
            position="relative"
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            items="center"
            justify="center"
          >
            <text fontSize={10} color={C.textDim}>
              {label}
            </text>
            <view
              position="absolute"
              {...pos}
              bg={c}
              hover:bg={'#ff0000'}
              rounded={6}
              w={10}
              h={10}
              p={6}
              display="flex"
              flexDir="col"
              gap={4}
            ></view>
          </view>
        ))}
      </view>

      <view
        h={140}
        position="relative"
        bg={C.surface2}
        rounded={8}
        border={1}
        borderColor={C.border}
        display="flex"
        items="center"
        justify="center"
      >
        <view fontSize={12} color={C.textDim}>
          relative container bg
        </view>
        <view
          position="absolute"
          top={10}
          left={10}
          right={10}
          bottom={10}
          bg="#e2a52e10"
          rounded={8}
          display="flex"
          items="center"
          justify="center"
        >
          <AbsClickCounter color={C.accent} />
        </view>
      </view>
    </view>
  );
}

export function LayoutPage() {
  const [showVisibility, setShowVisibility] = useState(false);
  const [showDisplay, setShowDisplay] = useState(false);
  const [gap, setGap] = useState(8);
  const [padding, setPadding] = useState(12);

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
          Layout Lab
        </view>
        <view fontSize={12} color={C.textMuted}>
          Flex, nesting, borders, rounding, opacity, visibility
        </view>
      </view>

      <view display="flex" flexDir="col" gap={24} p={24}>
        <view display="flex" flexDir="col" gap={12}>
          <view fontSize={14} fontWeight={700} color={C.text}>
            Flexbox — justify variants
          </view>
          {(['center', 'between', 'around', 'evenly'] as const).map((j) => (
            <view key={j} display="flex" flexDir="col" gap={4}>
              <view fontSize={11} fontWeight={600} color={C.textMuted}>
                justify="{j}"
              </view>
              <view
                display="flex"
                flexDir="row"
                justify={j}
                bg={C.surface2}
                rounded={8}
                p={12}
                border={1}
                borderColor={C.border}
              >
                {[C.accent, C.primary, C.success, C.warning].map((c, i) => (
                  <view key={i} w={36} h={36} bg={c} rounded={4} />
                ))}
              </view>
            </view>
          ))}
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Flexbox — items variants
          </text>
          <view display="flex" flexDir="row" gap={12}>
            {(['start', 'center', 'end', 'stretch'] as const).map((a) => (
              <view key={a} display="flex" flexDir="col" gap={4} flex={1}>
                <view fontSize={11} fontWeight={600} color={C.textMuted}>
                  items="{a}"
                </view>
                <view
                  display="flex"
                  flexDir="row"
                  items={a}
                  bg={C.surface2}
                  rounded={8}
                  p={10}
                  h={70}
                  border={1}
                  borderColor={C.border}
                  gap={4}
                >
                  {a === 'stretch' ? (
                    <>
                      <view w={24} bg={C.accent} rounded={4} />
                      <view w={24} bg={C.primary} rounded={4} />
                      <view w={24} bg={C.success} rounded={4} />
                    </>
                  ) : (
                    <>
                      <view w={24} h={24} bg={C.accent} rounded={4} />
                      <view w={24} h={36} bg={C.primary} rounded={4} />
                      <view w={24} h={16} bg={C.success} rounded={4} />
                    </>
                  )}
                </view>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Per-corner border-radius
          </text>
          <view display="flex" flexDir="row" gap={12} items="center">
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.accent} roundedTL={24} />
              <view fontSize={10} color={C.textMuted}>
                TL
              </view>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.primary} roundedTR={24} />
              <view fontSize={10} color={C.textMuted}>
                TR
              </view>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.success} roundedBR={24} />
              <view fontSize={10} color={C.textMuted}>
                BR
              </view>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.warning} roundedBL={24} />
              <view fontSize={10} color={C.textMuted}>
                BL
              </view>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.accent} roundedTL={24} roundedBR={24} />
              <view fontSize={10} color={C.textMuted}>
                TL+BR
              </view>
            </view>
            <view display="flex" flexDir="col" items="center" gap={4}>
              <view w={60} h={60} bg={C.danger} roundedTR={24} roundedBL={24} />
              <view fontSize={10} color={C.textMuted}>
                TR+BL
              </view>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view fontSize={14} fontWeight={700} color={C.text}>
            Per-side borders
          </view>
          <view display="flex" flexDir="row" gap={12} items="center">
            {[
              { side: 'Top', prop: { borderTop: 3 }, color: C.accentHi },
              { side: 'Right', prop: { borderRight: 3 }, color: C.primaryHi },
              { side: 'Bottom', prop: { borderBottom: 3 }, color: C.successHi },
              { side: 'Left', prop: { borderLeft: 3 }, color: C.warningHi },
              { side: 'All', prop: { border: 2 }, color: C.accentHi },
            ].map(({ side, prop, color }) => (
              <view
                key={side}
                display="flex"
                flexDir="col"
                items="center"
                gap={4}
              >
                <view
                  w={60}
                  h={60}
                  bg={C.surface2}
                  rounded={8}
                  borderColor={color}
                  {...prop}
                />
                <text fontSize={10} color={C.textMuted}>
                  {side}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view fontSize={14} fontWeight={700} color={C.text}>
            Opacity scale
          </view>
          <view display="flex" flexDir="row" gap={8} items="center">
            {[1, 0.8, 0.6, 0.4, 0.2, 0.1].map((op) => (
              <view
                key={op}
                display="flex"
                flexDir="col"
                items="center"
                gap={4}
              >
                <view
                  w={52}
                  h={52}
                  bg={C.accent}
                  rounded={8}
                  opacity={op}
                  display="flex"
                  items="center"
                  justify="center"
                >
                  <text fontSize={11} fontWeight={700} color="#fff">
                    {op}
                  </text>
                </view>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="col" gap={4}>
            <view fontSize={14} fontWeight={700} color={C.text}>
              Transforms
            </view>
            <view fontSize={12} color={C.textMuted}>
              translate, rotate, scale, and hover:scale without changing layout
            </view>
          </view>
          <view
            display="flex"
            flexDir="row"
            gap={16}
            p={18}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            {[
              {
                label: 'hover scale',
                props: { scale: 1, 'hover:scale': 1.16 },
                color: C.accent,
              },
              {
                label: 'rotated',
                props: { rotate: -8, 'hover:rotate': 8 },
                color: C.primary,
              },
              {
                label: 'translated',
                props: { translate: [0, 0], 'hover:translate': [10, -8] },
                color: C.success,
              },
            ].map(({ label, props, color }) => (
              <view
                key={label}
                w={120}
                h={72}
                bg={color}
                rounded={8}
                cursor="pointer"
                display="flex"
                items="center"
                justify="center"
                {...(props as any)}
              >
                <text fontSize={12} fontWeight={800} color="#ffffff">
                  {label}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="row" items="center" gap={20}>
            <view fontSize={14} fontWeight={700} color={C.text}>
              Dynamic gap / padding
            </view>
            <view display="flex" flexDir="row" items="center" gap={8}>
              <button
                onClick={() => setGap((g) => Math.max(2, g - 2))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  gap−
                </text>
              </button>
              <text fontSize={12} color={C.accentHi}>
                gap={gap}
              </text>
              <button
                onClick={() => setGap((g) => Math.min(40, g + 2))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  gap+
                </text>
              </button>
              <button
                onClick={() => setPadding((p) => Math.max(4, p - 4))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  p−
                </text>
              </button>
              <text fontSize={12} color={C.primaryHi}>
                p={padding}
              </text>
              <button
                onClick={() => setPadding((p) => Math.min(40, p + 4))}
                px={10}
                py={4}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={4}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={13} color={C.text}>
                  p+
                </text>
              </button>
            </view>
          </view>
          <view
            display="flex"
            flexDir="row"
            gap={gap}
            p={padding}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            {['A', 'B', 'C', 'D', 'E'].map((l, i) => (
              <view
                key={l}
                flex={1}
                p={padding}
                bg={[C.accent, C.primary, C.success, C.warning, C.danger][i]}
                rounded={8}
                display="flex"
                items="center"
                justify="center"
              >
                <text fontSize={16} fontWeight={800} color={C.text}>
                  {l}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <view display="flex" flexDir="col" gap={4}>
            <view fontSize={14} fontWeight={700} color={C.text}>
              Buttons
            </view>
            <view fontSize={12} color={C.textMuted}>
              Various button configurations and property combinations
            </view>
          </view>

          <view display="flex" flexDir="col" gap={12}>
            <view display="flex" flexDir="row" gap={12}>
              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Default
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    No properties set
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button bg={C.accent} cursor="pointer" hover:bg={C.accentDim}>
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    With Padding
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    px: 12 | py: 6
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Uniform Padding
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    padding: 12
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    p={12}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>
            </view>

            <view display="flex" flexDir="row" gap={12}>
              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Rounded
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    rounded: 8 | px: 12 | py: 6
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    rounded={8}
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view
                display="flex"
                flexDir="col"
                gap={8}
                flex={1}
                p={12}
                bg={C.surface2}
                rounded={12}
                border={1}
                borderColor={C.border}
              >
                <view display="flex" flexDir="col" gap={2}>
                  <text fontSize={13} fontWeight={600} color={C.accentHi}>
                    Flex Centered
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    flex | px: 12 | py: 6 | rounded: 8
                  </text>
                </view>
                <view
                  display="flex"
                  items="center"
                  justify="center"
                  p={8}
                  bg={C.surface}
                  rounded={8}
                >
                  <button
                    display="flex"
                    flexDir="row"
                    justify="center"
                    rounded={8}
                    px={12}
                    py={6}
                    bg={C.accent}
                    cursor="pointer"
                    hover:bg={C.accentDim}
                  >
                    button text
                  </button>
                </view>
              </view>

              <view flex={1} />
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Deep nesting (6 levels)
          </text>
          <view
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <view
              p={14}
              bg={C.surface3}
              rounded={8}
              border={1}
              borderColor={C.borderHi}
            >
              <view
                p={12}
                bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.primaryDim}
              >
                <view
                  p={10}
                  bg={C.accentDark}
                  rounded={8}
                  border={1}
                  borderColor={C.accent}
                >
                  <view
                    p={8}
                    bg={C.accentDim}
                    rounded={8}
                    border={1}
                    borderColor={C.accentHi}
                  >
                    <view
                      p={6}
                      bg={C.accent}
                      rounded={4}
                      display="flex"
                      items="center"
                      justify="center"
                    >
                      <text fontSize={12} fontWeight={700} color="#fff">
                        6 levels
                      </text>
                    </view>
                  </view>
                </view>
              </view>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              Cursor kinds
            </text>
          </view>
          <view display="flex" flexDir="row" gap={8}>
            {(
              [
                'default',
                'pointer',
                'text',
                'crosshair',
                'not-allowed',
                'grab',
              ] as const
            ).map((cur) => (
              <view
                key={cur}
                px={14}
                py={10}
                bg={C.surface2}
                hover:bg={C.surface3}
                active:bg={C.surface4}
                rounded={8}
                border={1}
                borderColor={C.border}
                hover:borderColor={C.accentHi}
                cursor={cur}
              >
                <text fontSize={12} color={C.textDim} hover:color={C.text}>
                  {cur}
                </text>
              </view>
            ))}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={12}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              display prop
            </text>
            <button
              onClick={() => setShowDisplay((s) => !s)}
              px={14}
              py={6}
              bg={showDisplay ? C.accentDim : C.surface3}
              hover:bg={showDisplay ? C.accent : C.surface4}
              rounded={8}
              border={1}
              borderColor={showDisplay ? C.accent : C.border}
              cursor="pointer"
            >
              <text
                fontSize={12}
                fontWeight={600}
                color={showDisplay ? C.accentHi : C.textMuted}
              >
                {showDisplay ? 'Hide it' : 'Show it'}
              </text>
            </button>
          </view>
          <view
            display={showDisplay ? 'flex' : 'none'}
            p={14}
            bg={C.primaryDark}
            rounded={8}
            border={1}
            borderColor={C.primary}
          >
            <text fontSize={14} color={C.primaryHi} fontWeight={600}>
              👁 Now you see me via display!
            </text>
          </view>
          <view
            display={'flex'}
            p={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <text fontSize={14} color={C.textMuted}>
              Click the button to toggle with display.
            </text>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={12}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              visible prop
            </text>
            <button
              onClick={() => setShowVisibility((s) => !s)}
              px={14}
              py={6}
              bg={showVisibility ? C.accentDim : C.surface3}
              hover:bg={showVisibility ? C.accent : C.surface4}
              rounded={8}
              border={1}
              borderColor={showVisibility ? C.accent : C.border}
              cursor="pointer"
            >
              <text
                fontSize={12}
                fontWeight={600}
                color={showVisibility ? C.accentHi : C.textMuted}
              >
                {showVisibility ? 'Hide it' : 'Reveal it'}
              </text>
            </button>
          </view>
          <view
            visibility={showVisibility ? 'visible' : 'hidden'}
            p={14}
            bg={C.accentDark}
            rounded={8}
            border={1}
            borderColor={C.accent}
          >
            <text fontSize={14} color={C.accentHi} fontWeight={600}>
              👁 Now you see me! (visibility)
            </text>
          </view>
          <view
            p={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <text fontSize={14} color={C.textMuted}>
              Click the button to toggle visibility.
            </text>
          </view>
        </view>

        <Divider />

        <InlineFlowDemo />

        <Divider />

        <TextAlignDemo />

        <Divider />

        <ScrollDemo />

        <Divider />

        <AbsolutePositioningDemo />
      </view>
    </view>
  );
}

function InlineFlowDemo() {
  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <text fontSize={14} fontWeight={700} color={C.text}>
          Inline flow (text + text elements)
        </text>
        <text fontSize={12} color={C.textMuted}>
          Bare text and consecutive text elements should sit on one line. Drag
          to select across them.
        </text>
      </view>

      <view
        selectable
        p={16}
        bg={C.surface2}
        rounded={8}
        border={1}
        textAlign="center"
        borderColor={C.border}
        fontSize={14}
        color={C.text}
      >
        Hello{' '}
        <text color={C.accentHi} fontWeight={700}>
          world
        </text>
        <text color={C.textMuted}> from </text>
        <text color={C.successHi} fontWeight={700}>
          uzumaki
        </text>
        !
      </view>

      <view
        selectable
        p={16}
        bg={C.surface2}
        rounded={8}
        border={1}
        borderColor={C.border}
        display="flex"
        flexDir="col"
        gap={8}
        fontSize={13}
        color={C.text}
      >
        <view>
          inline run: <text color={C.accentHi}>red</text>{' '}
          <text color={C.primaryHi}>green</text>{' '}
          <text color={C.successHi}>blue</text> end.
        </view>
        <view>
          mixed siblings:
          <text color={C.warningHi} fontWeight={700}>
            {' '}
            tag-A{' '}
          </text>
          plain
          <text color={C.accentHi} fontWeight={700}>
            {' '}
            tag-B{' '}
          </text>
          plain again.
        </view>
      </view>
    </view>
  );
}

function ScrollDemo() {
  return (
    <view display="flex" flexDir="col" gap={12}>
      <view display="flex" flexDir="col" gap={4}>
        <text fontSize={14} fontWeight={700} color={C.text}>
          Scroll props
        </text>
        <text fontSize={12} color={C.textMuted}>
          `scroll` enables auto overflow on both axes, while `scrollX` and
          `scrollY` let you opt into each axis separately.
        </text>
      </view>

      <view display="flex" flexDir="col" gap={16}>
        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scroll
          </text>
          <view
            h={200}
            scroll
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="col"
            gap={8}
          >
            {Array.from({ length: 12 }, (_, i) => (
              <view key={i} display="flex" flexDir="row" gap={10}>
                {Array.from({ length: 6 }, (_, j) => (
                  <view
                    key={`${i}-${j}`}
                    w={140}
                    flexShrink={0}
                    p={10}
                    bg={(i + j) % 2 === 0 ? C.surface3 : C.surface4}
                    rounded={6}
                    display="flex"
                    items="center"
                    justify="between"
                  >
                    <text fontSize={12} fontWeight={600} color={C.text}>
                      Row {i + 1}
                    </text>
                    <text fontSize={11} color={C.textMuted}>
                      Col {j + 1}
                    </text>
                  </view>
                ))}
              </view>
            ))}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scrollX
          </text>
          <view
            h={108}
            scrollX
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="row"
            scrollbarWidth={4}
            scrollbarColor={C.accentDim}
            scrollbarHoverColor={C.warningDim}
            gap={10}
          >
            {Array.from({ length: 10 }, (_, i) => (
              <view
                key={i}
                w={140}
                h={80}
                textWrap="nowrap"
                bg={i % 2 === 0 ? C.primary : C.accent}
                rounded={8}
                p={10}
                display="flex"
                flexDir="col"
                justify="between"
              >
                <text fontSize={12} fontWeight={700} color={C.text}>
                  Card #{i + 1}
                </text>
                <text fontSize={11} color={C.textMuted}>
                  Horizontal overflow only
                </text>
              </view>
            ))}
            <button
              w={140}
              h={80}
              flexShrink={0}
              bg={C.accent}
              hover:bg={C.accentHi}
              rounded={8}
              px={10}
              py={10}
              cursor="pointer"
              display="flex"
              items="center"
              justify="center"
            >
              <text fontSize={12} fontWeight={700} color="#fff">
                Tab to me
              </text>
            </button>
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scrollY
          </text>
          <view
            h={220}
            scrollY
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="col"
            gap={10}
            scrollbarColor={C.accentDim}
            scrollbarHoverColor={C.warningDim}
          >
            {Array.from({ length: 14 }, (_, i) => (
              <view
                key={i}
                p={12}
                bg={i % 2 === 0 ? C.success : C.warning}
                rounded={8}
                display="flex"
                flexDir="col"
                gap={4}
              >
                <text fontSize={12} fontWeight={700} color={C.text}>
                  Log entry #{i + 1}
                </text>
                <text fontSize={10} color={C.textMuted}>
                  Vertical overflow only
                </text>
              </view>
            ))}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={12} fontWeight={600} color={C.textMuted}>
            scrollbar styling
          </text>
          <text fontSize={11} color={C.textMuted}>
            scrollbarWidth / Color / HoverColor / TrackColor / Radius
          </text>
          <view
            h={220}
            scrollY
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            p={12}
            display="flex"
            flexDir="col"
            gap={10}
            scrollbarHoverColor={C.primary}
            scrollbarRadius={5}
          >
            {Array.from({ length: 14 }, (_, i) => (
              <view
                key={i}
                p={12}
                bg={i % 2 === 0 ? C.surface3 : C.surface4}
                rounded={8}
                display="flex"
                flexDir="col"
                gap={4}
              >
                <text fontSize={12} fontWeight={700} color={C.text}>
                  Item #{i + 1}
                </text>
                <text fontSize={10} color={C.textMuted}>
                  Hover the thumb to see the hover color
                </text>
              </view>
            ))}
          </view>
        </view>
      </view>
    </view>
  );
}
