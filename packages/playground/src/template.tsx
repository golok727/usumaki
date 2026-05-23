import { useEffect, useState } from 'react';
import { useWindow } from 'uzumaki-react';
import { C } from './theme';

const uzumakiLogo = Uz.path.resource('assets/logo.svg');
const reactLogo = Uz.path.resource('assets/react.svg');
const SPIN_DEGREES_PER_SECOND = 90;

export function TemplateApp() {
  const window = useWindow();
  const [count, setCount] = useState(0);
  const [spin, setSpin] = useState(0);

  useEffect(() => {
    let frame = 0;
    const tick = (timestamp: number) => {
      setSpin(((timestamp * SPIN_DEGREES_PER_SECOND) / 1000) % 360);
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frame);
  }, [window]);

  return (
    <view
      display="flex"
      flexDir="col"
      w="full"
      h="full"
      items="center"
      justify="center"
      gap={22}
    >
      <view display="flex" flexDir="row" items="center" gap={'3rem'}>
        <image rotate={spin} src={uzumakiLogo} w={116} h={116} />
        <image rotate={spin} src={reactLogo} w={128} h={116} />
      </view>
      <view display="flex" flexDir="row" items="center" gap={20}>
        <view fontSize={34} fontWeight={700} color={C.accent}>
          Uzumaki
        </view>
        <view fontSize={30} fontWeight={700} color={C.textDim}>
          💖
        </view>
        <view fontSize={34} fontWeight={700} color="#61dafb">
          React
        </view>
      </view>
      <text fontSize={18} color={C.textSub}>
        Count: {count}
      </text>
      <button
        onClick={() => setCount((c) => c + 1)}
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
          Increment
        </text>
      </button>
    </view>
  );
}
