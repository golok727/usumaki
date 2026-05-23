import { C } from './theme';

export function Divider() {
  return <view h={1} bg={C.border} w="full" />;
}

export function Badge({
  label,
  color,
  bg,
}: {
  label: string;
  color: string;
  bg: string;
}) {
  return (
    <view px={8} py={3} bg={bg} rounded={8}>
      <text textWrap="nowrap" fontSize={11} fontWeight={600} color={color}>
        {label}
      </text>
    </view>
  );
}

export function ProgressBar({
  value,
  color,
  trackColor,
  label,
  showPct = true,
}: {
  value: number;
  color: string;
  trackColor: string;
  label: string;
  showPct?: boolean;
}) {
  const pct = Math.max(0, Math.min(100, value));
  return (
    <view display="flex" flexDir="col" gap={5} w="full">
      <view display="flex" flexDir="row" items="center" justify="between">
        <view fontSize={12} color={C.textMuted}>
          {label}
        </view>
        {showPct && (
          <view fontSize={12} fontWeight={700} color={color}>
            {pct.toFixed(1)}%
          </view>
        )}
      </view>
      <view w="full" h={5} bg={trackColor} rounded={4}>
        <view w={`${pct}%`} h={5} bg={color} rounded={4} />
      </view>
    </view>
  );
}

export function StatCard({
  label,
  value,
  sub,
  color,
}: {
  label: string;
  value: string;
  sub: string;
  color: string;
}) {
  return (
    <view
      textWrap="nowrap"
      flex={1}
      p={16}
      bg={C.surface2}
      rounded={8}
      border={1}
      borderColor={C.border}
      display="flex"
      flexDir="col"
      gap={7}
    >
      <view fontSize={10} fontWeight={700} color={C.textMuted}>
        {label.toUpperCase()}
      </view>
      <view fontSize={26} fontWeight={800} color={color}>
        {value}
      </view>
      <view fontSize={11} color={C.textMuted}>
        {sub}
      </view>
    </view>
  );
}
