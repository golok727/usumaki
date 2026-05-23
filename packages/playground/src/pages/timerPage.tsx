import { useEffect, useState } from 'react';
import { C } from '../theme';

export function TimerPage() {
  const [count500, setCount500] = useState(0);
  const [count1s, setCount1s] = useState(0);
  const [count2s, setCount2s] = useState(0);
  const [count5s, setCount5s] = useState(0);
  const [count10s, setCount10s] = useState(0);

  useEffect(() => {
    const id500 = setInterval(() => setCount500((prev) => prev + 1), 500);
    const id1s = setInterval(() => setCount1s((prev) => prev + 1), 1000);
    const id2s = setInterval(() => setCount2s((prev) => prev + 1), 2000);
    const id5s = setInterval(() => setCount5s((prev) => prev + 1), 5000);
    const id10s = setInterval(() => setCount10s((prev) => prev + 1), 10_000);
    return () => {
      clearInterval(id500);
      clearInterval(id1s);
      clearInterval(id2s);
      clearInterval(id5s);
      clearInterval(id10s);
    };
  }, []);

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
        gap={8}
        borderBottom="1"
        borderColor={C.border}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Timer
        </view>
        <view fontSize={12} color={C.textMuted}>
          Multiple counters with different intervals
        </view>
      </view>

      <view display="flex" flexDir="col" gap={16} p={24}>
        <view
          display="flex"
          flexDir="col"
          items="center"
          justify="center"
          p={24}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={48} fontWeight={800} color={C.accentHi}>
            {count500}
          </text>
          <text fontSize={13} color={C.textMuted} mt={4}>
            500ms interval
          </text>
        </view>

        <view
          display="flex"
          flexDir="col"
          items="center"
          justify="center"
          p={24}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={48} fontWeight={800} color={C.primaryHi}>
            {count1s}
          </text>
          <text fontSize={13} color={C.textMuted} mt={4}>
            1s interval
          </text>
        </view>

        <view
          display="flex"
          flexDir="col"
          items="center"
          justify="center"
          p={24}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={48} fontWeight={800} color={C.successHi}>
            {count2s}
          </text>
          <text fontSize={13} color={C.textMuted} mt={4}>
            2s interval
          </text>
        </view>

        <view
          display="flex"
          flexDir="col"
          items="center"
          justify="center"
          p={24}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={48} fontWeight={800} color={C.warningHi}>
            {count5s}
          </text>
          <text fontSize={13} color={C.textMuted} mt={4}>
            5s interval
          </text>
        </view>

        <view
          display="flex"
          flexDir="col"
          items="center"
          justify="center"
          p={24}
          bg={C.surface2}
          rounded={12}
          border="1"
          borderColor={C.border}
        >
          <text fontSize={48} fontWeight={800} color={C.dangerHi}>
            {count10s}
          </text>
          <text fontSize={13} color={C.textMuted} mt={4}>
            10s interval
          </text>
        </view>
      </view>
    </view>
  );
}
