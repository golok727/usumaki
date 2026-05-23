import { useState, useEffect } from 'react';
import { C } from '../theme';
import { Divider, Badge, ProgressBar, StatCard } from '../components';

export function DashboardPage() {
  const [{ tick, cpu, mem, net, gpu, disk }, setCpuStatStat] = useState({
    tick: 0,
    cpu: 42,
    mem: 67,
    net: 31,
    gpu: 78,
    disk: 54,
  });

  const [log, setLog] = useState<string[]>([
    'Renderer initialized',
    'wgpu device acquired',
    'Vello scene created',
    'React tree mounted',
  ]);

  useEffect(() => {
    const id = setInterval(() => {
      setCpuStatStat((prev) => ({
        ...prev,
        tick: prev.tick + 1,
        cpu: Math.max(4, Math.min(97, prev.cpu + (Math.random() - 0.48) * 8)),
        mem: Math.max(20, Math.min(93, prev.mem + (Math.random() - 0.5) * 3)),
        net: Math.max(0, Math.min(100, prev.net + (Math.random() - 0.5) * 18)),
        gpu: Math.max(30, Math.min(99, prev.gpu + (Math.random() - 0.5) * 6)),
        disk: Math.max(10, Math.min(80, prev.disk + (Math.random() - 0.5) * 2)),
      }));

      setLog((prev) => {
        const events = [
          'Render pass started',
          'Layout computed',
          'GPU flush',
          'Event polled',
          'State reconciled',
          'Paint scheduled',
          'Frame submitted',
          'Texture uploaded',
          'Shader compiled',
          'Buffer mapped',
          'Glyph rasterized',
          'Path stroked',
        ];
        const ev = events[Math.floor(Math.random() * events.length)] as string;
        return [ev, ...prev.slice(0, 14)];
      });
    }, 500);
    return () => clearInterval(id);
  }, []);

  const fps = 60;

  return (
    <view
      display="flex"
      flexDir="col"
      gap={0}
      h="full"
      scrollY
      scrollbarWidth={8}
      scrollbarRadius={5}
    >
      <view
        display="flex"
        flexDir="row"
        items="center"
        justify="between"
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <view display="flex" flexDir="col" gap={8}>
          <view fontSize={20} fontWeight={800} color={C.text}>
            System Dashboard
          </view>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <view
              px={8}
              py={2}
              bg={C.accentDim}
              rounded={4}
              textWrap="nowrap"
              fontSize={10}
              fontWeight={700}
              color={C.accentHi}
            >
              DEMO MODE
            </view>
            <view fontSize={12} color={C.textMuted}>
              Fake data for demonstration
            </view>
          </view>
        </view>
        <view display="flex" flexDir="row" items="center" gap={10}>
          <view
            display="flex"
            flexDir="row"
            items="center"
            gap={6}
            px={12}
            py={6}
            bg={C.successDim}
            rounded={8}
          >
            <text
              fontSize={12}
              textWrap="nowrap"
              fontWeight={600}
              color={C.successHi}
            >
              LIVE
            </text>
          </view>
          <view
            px={12}
            py={6}
            bg={C.surface3}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <text fontSize={12} color={C.textMuted}>
              tick #{tick} · frame ~{tick * fps}
            </text>
          </view>
        </view>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <view display="flex" flexDir="row" gap={12}>
          <StatCard
            label="Rendered Frames"
            value={`${(tick * fps).toLocaleString()}`}
            sub={`~${fps} fps target`}
            color={C.accentHi}
          />
          <StatCard
            label="React Nodes"
            value={`${148 + (tick % 12)}`}
            sub="in tree"
            color={C.primaryHi}
          />
          <StatCard
            label="Heap Used"
            value={`${(mem * 0.72).toFixed(1)} MB`}
            sub="managed runtime"
            color={C.successHi}
          />
          <StatCard
            label="Events Fired"
            value={`${(tick * 6).toLocaleString()}`}
            sub="total dispatched"
            color={C.warningHi}
          />
        </view>

        <view display="flex" flexDir="row" gap={12}>
          <view
            flex={3}
            minW={0}
            p={20}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            flexDir="col"
            gap={16}
          >
            <view fontSize={14} fontWeight={700} color={C.text}>
              Resource Usage
            </view>
            <ProgressBar
              label="CPU"
              value={cpu}
              color={cpu > 80 ? C.danger : (cpu > 60 ? C.warning : C.accentHi)}
              trackColor={C.surface4}
            />
            <ProgressBar
              label="Memory"
              value={mem}
              color={mem > 80 ? C.danger : C.primary}
              trackColor={C.surface4}
            />
            <ProgressBar
              label="Network I/O"
              value={net}
              color={C.success}
              trackColor={C.surface4}
            />
            <ProgressBar
              label="GPU"
              value={gpu}
              color={gpu > 85 ? C.warning : C.primary}
              trackColor={C.surface4}
            />
            <ProgressBar
              label="Disk"
              value={disk}
              color={C.textSub}
              trackColor={C.surface4}
            />
          </view>

          <view
            flex={2}
            minW={0}
            p={20}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            display="flex"
            flexDir="col"
            gap={12}
          >
            <view fontSize={14} fontWeight={700} color={C.text}>
              CPU Bars (last 20 ticks)
            </view>
            <view display="flex" flexDir="row" flex={1} gap={2} h={80} minW={0}>
              {Array.from({ length: 20 }, (_, i) => {
                const age = 19 - i;
                const h = 20 + Math.abs(Math.sin((tick - age) * 0.7 + i) * 60);
                const isHot = h > 65;
                const barHeight = Math.round((h / 100) * 80);
                return (
                  <view
                    key={i}
                    flex={1}
                    minW={0}
                    display="flex"
                    flexDir="col"
                    justify="end"
                    h="full"
                  >
                    <view
                      w="full"
                      h={barHeight}
                      bg={isHot ? C.danger : (age < 3 ? C.accentHi : C.accentDim)}
                      rounded={4}
                      opacity={1 - age * 0.03}
                    />
                  </view>
                );
              })}
            </view>
            <Divider />
            <view display="flex" flexDir="col" gap={8}>
              <view
                display="flex"
                flexDir="row"
                items="center"
                justify="between"
              >
                <view fontSize={12} color={C.textMuted}>
                  Current
                </view>
                <view
                  fontSize={13}
                  fontWeight={700}
                  color={cpu > 80 ? C.dangerHi : C.accentHi}
                >
                  {cpu.toFixed(1)}%
                </view>
              </view>
              <view
                display="flex"
                flexDir="row"
                items="center"
                justify="between"
              >
                <view fontSize={12} color={C.textMuted}>
                  Peak
                </view>
                <view fontSize={13} fontWeight={700} color={C.warning}>
                  97.3%
                </view>
              </view>
              <view
                display="flex"
                flexDir="row"
                items="center"
                justify="between"
              >
                <view fontSize={12} color={C.textMuted}>
                  Avg
                </view>
                <view fontSize={13} fontWeight={700} color={C.textSub}>
                  51.2%
                </view>
              </view>
            </view>
          </view>
        </view>

        <view
          display="flex"
          flexDir="col"
          p={20}
          bg={C.surface}
          rounded={8}
          border={1}
          borderColor={C.border}
        >
          <view
            display="flex"
            flexDir="row"
            items="center"
            justify="between"
            mb={12}
          >
            <text fontSize={14} fontWeight={700} color={C.text}>
              Render Event Log
            </text>
            <Badge label="STREAMING" color={C.successHi} bg={C.successDim} />
          </view>
          <view scroll h={160} display="flex" flexDir="col">
            {log.map((ev, i) => (
              <view
                key={i}
                w="full"
                display="flex"
                flexDir="row"
                items="center"
                gap={10}
                py={6}
                borderBottom={1}
                borderColor={C.border}
              >
                <text fontSize={10} color={C.textMuted} fontWeight={700}>
                  {String(log.length - i).padStart(3, '0')}
                </text>
                <view
                  w={6}
                  h={6}
                  bg={i === 0 ? C.success : C.surface4}
                  rounded={4}
                />
                <text
                  fontSize={12}
                  color={i === 0 ? C.successHi : C.textMuted}
                  fontWeight={i === 0 ? 600 : 400}
                >
                  {ev}
                </text>
              </view>
            ))}
          </view>
        </view>
      </view>
    </view>
  );
}
