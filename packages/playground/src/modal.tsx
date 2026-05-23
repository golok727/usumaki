import { useId, useState } from 'react';
import { UzElement, type WindowTheme } from 'uzumaki';
import { useWindow } from 'uzumaki-react';
import { C } from './theme';
import { setTheme } from './useTheme';

function ModalCounter() {
  const [count, setCount] = useState(0);
  return (
    <view display="flex" flexDir="row" items="center" gap={10}>
      <button
        onClick={() => setCount((c) => c - 1)}
        w={32}
        h={32}
        bg={C.surface4}
        hover:bg={C.borderHi}
        rounded={6}
        display="flex"
        items="center"
        justify="center"
        cursor="pointer"
      >
        <text fontSize={16} fontWeight={700} color={C.text}>
          -
        </text>
      </button>
      <text fontSize={20} fontWeight={700} color={C.accentHi}>
        {count}
      </text>
      <button
        onClick={() => setCount((c) => c + 1)}
        w={32}
        h={32}
        bg={C.surface4}
        hover:bg={C.borderHi}
        rounded={6}
        display="flex"
        items="center"
        justify="center"
        cursor="pointer"
      >
        <text fontSize={16} fontWeight={700} color={C.text}>
          +
        </text>
      </button>
    </view>
  );
}

const THEME_OPTIONS: { label: string; value: WindowTheme }[] = [
  { label: 'System', value: 'system' },
  { label: 'Dark', value: 'dark' },
  { label: 'Light', value: 'light' },
];

export function Modal({ onClose }: { onClose: () => void }) {
  const modalId = useId();
  const window = useWindow();
  const [preference, setPreference] = useState<WindowTheme>(window.theme);
  return (
    <view
      id={modalId}
      position="absolute"
      top={0}
      left={0}
      right={0}
      bottom={0}
      bg="#00000088"
      display="flex"
      items="center"
      justify="center"
      onClick={(ev) => {
        if (ev.target instanceof UzElement && ev.target.id === modalId)
          onClose();
      }}
    >
      <view
        w={420}
        bg={C.surface}
        rounded={16}
        border={1}
        borderColor={C.borderHi}
        display="flex"
        flexDir="col"
      >
        <view
          px={24}
          py={16}
          borderBottom={1}
          borderColor={C.border}
          display="flex"
          flexDir="row"
          items="center"
          justify="between"
        >
          <text fontSize={16} fontWeight={800} color={C.text}>
            Settings
          </text>
        </view>

        <view px={24} py={20} display="flex" flexDir="col" gap={18}>
          <view display="flex" flexDir="col" gap={6}>
            <text fontSize={13} fontWeight={600} color={C.text}>
              Theme
            </text>
            <view display="flex" flexDir="row" gap={8}>
              {THEME_OPTIONS.map(({ label, value }) => {
                const selected = preference === value;
                return (
                  <view
                    key={value}
                    onClick={() => {
                      setTheme(window, value);
                      setPreference(value);
                    }}
                    px={14}
                    py={7}
                    bg={selected ? C.accentDim : C.surface3}
                    hover:bg={selected ? C.accent : C.surface4}
                    rounded={6}
                    cursor="pointer"
                    border={1}
                    borderColor={selected ? C.accent : C.border}
                  >
                    <text
                      fontSize={12}
                      fontWeight={selected ? '700' : '400'}
                      color={selected ? C.accentHi : C.textSub}
                    >
                      {label}
                    </text>
                  </view>
                );
              })}
            </view>
          </view>

          <view display="flex" flexDir="col" gap={6}>
            <text fontSize={13} fontWeight={600} color={C.text}>
              Counter
            </text>
            <ModalCounter />
          </view>
        </view>

        <view
          px={24}
          py={14}
          borderTop={1}
          borderColor={C.border}
          display="flex"
          flexDir="row"
          justify="end"
          gap={8}
        >
          <button
            onClick={onClose}
            px={16}
            py={8}
            bg={C.surface3}
            hover:bg={C.surface4}
            rounded={8}
            cursor="pointer"
          >
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Close
            </text>
          </button>
        </view>
      </view>
    </view>
  );
}
