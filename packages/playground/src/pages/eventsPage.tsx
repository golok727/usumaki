import { useState, useCallback } from 'react';
import { C } from '../theme';
import { Badge } from '../components';

export function EventsPage() {
  const [eventLog, setEventLog] = useState<
    Array<{ type: string; ts: number; n: number }>
  >([]);
  const [clicks, setClicks] = useState(0);
  const [downs, setDowns] = useState(0);
  const [ups, setUps] = useState(0);
  const [seq, setSeq] = useState(0);
  const [hovering, setHovering] = useState(false);
  const [pos, setPos] = useState<{
    x: number;
    y: number;
    localX: number;
    localY: number;
  } | null>(null);
  const [filtered, setFiltered] = useState('');
  const [blocked, setBlocked] = useState(0);
  const [committed, setCommitted] = useState('');
  const [checked, setChecked] = useState(false);

  const push = useCallback((type: string) => {
    setSeq((s) => {
      const n = s + 1;
      setEventLog((l) => [{ type, ts: Date.now(), n }, ...l.slice(0, 59)]);
      return n;
    });
  }, []);

  const typeColor = (t: string) => {
    if (t === 'onClick') return C.accentHi;
    if (t === 'onMouseDown') return C.primaryHi;
    if (t === 'onBeforeInput') return C.warningHi;
    if (t === 'onCommit') return C.primaryHi;
    if (t === 'onMouseEnter' || t === 'onMouseLeave') return C.dangerHi;
    return C.successHi;
  };

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
        flexDir="col"
        gap={8}
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Events
        </view>
        <view fontSize={12} color={C.textMuted}>
          onClick · onMouseDown · onMouseUp · onMouseEnter · onMouseLeave ·
          onMouseMove · onBeforeInput · onCommit · hover:* · active:*
        </view>
      </view>

      <view display="flex" flexDir="col" gap={24} p={24}>
        <button
          onClick={() => {
            setClicks((c) => c + 1);
            push('onClick');
          }}
          onMouseDown={() => {
            setDowns((d) => d + 1);
            push('onMouseDown');
          }}
          onMouseUp={() => {
            setUps((u) => u + 1);
            push('onMouseUp');
          }}
          w="full"
          h={110}
          bg={C.surface2}
          hover:bg={C.surface3}
          active:bg={C.accentDark}
          rounded={8}
          border={2}
          borderColor={C.border}
          hover:borderColor={C.accent}
          active:borderColor={C.accentHi}
          display="flex"
          items="center"
          justify="center"
          cursor="pointer"
        >
          <view display="flex" flexDir="col" items="center" gap={4}>
            <text
              fontSize={18}
              fontWeight={700}
              color={C.textSub}
              hover:color={C.text}
            >
              Click / Press Here
            </text>
            <text fontSize={12} color={C.textMuted}>
              hover, active, onClick, onMouseDown, onMouseUp
            </text>
          </view>
        </button>

        <view display="flex" flexDir="row" gap={12}>
          {[
            {
              label: 'onClick',
              count: clicks,
              color: C.accentHi,
              bg: C.accentDark,
            },
            {
              label: 'onMouseDown',
              count: downs,
              color: C.primaryHi,
              bg: C.primaryDim,
            },
            {
              label: 'onMouseUp',
              count: ups,
              color: C.successHi,
              bg: C.successDim,
            },
            {
              label: 'Total Events',
              count: seq,
              color: C.warningHi,
              bg: C.warningDim,
            },
          ].map(({ label, count, color, bg }) => (
            <button
              key={label}
              flex={1}
              p={16}
              bg={C.surface2}
              rounded={8}
              border={1}
              borderColor={C.border}
              display="flex"
              flexDir="col"
              items="center"
              gap={6}
            >
              <view px={10} py={4} bg={bg} rounded={8}>
                <text fontSize={10} fontWeight={700} color={color}>
                  {label}
                </text>
              </view>
              <text fontSize={36} fontWeight={900} color={color}>
                {count}
              </text>
            </button>
          ))}
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            hover: / active: prop variants
          </text>
          <view display="flex" flexDir="row" gap={10}>
            {[
              {
                label: 'hover:bg',
                props: { bg: C.surface2, 'hover:bg': C.accent },
                // Bright fill on hover, so flip the label dark for contrast.
                textProps: { color: C.textSub, 'hover:color': C.bg },
              },
              {
                label: 'hover:opacity',
                props: { bg: C.surface2, 'hover:opacity': 0.5 },
                textProps: { color: C.textSub },
              },
              {
                label: 'active:bg',
                props: { bg: C.surface2, 'active:bg': C.success },
                textProps: { color: C.textSub, 'active:color': C.bg },
              },
              {
                label: 'all',
                props: {
                  bg: C.surface2,
                  'hover:bg': C.surface3,
                  'active:bg': C.accentDim,
                  border: 1,
                  borderColor: C.border,
                  'hover:borderColor': C.accentHi,
                  'active:borderColor': C.accentHi,
                },
                textProps: { color: C.textSub, 'hover:color': C.text },
              },
            ].map(({ label, props, textProps }) => {
              return (
                <button
                  key={label}
                  flex={1}
                  p={14}
                  rounded={8}
                  cursor="pointer"
                  display="flex"
                  items="center"
                  justify="center"
                  {...props}
                >
                  <text fontSize={12} {...textProps}>
                    {label}
                  </text>
                </button>
              );
            })}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            onMouseEnter / onMouseLeave / onMouseMove
          </text>
          <view
            onMouseEnter={() => {
              setHovering(true);
              push('onMouseEnter');
            }}
            onMouseLeave={() => {
              setHovering(false);
              push('onMouseLeave');
            }}
            onMouseMove={(e) =>
              setPos({
                x: Math.round(e.x),
                y: Math.round(e.y),
                localX: Math.round(e.localX),
                localY: Math.round(e.localY),
              })
            }
            h={90}
            rounded={8}
            border={2}
            borderColor={hovering ? C.accentHi : C.border}
            bg={C.surface2}
            display="flex"
            flexDir="col"
            items="center"
            justify="center"
            gap={4}
            cursor="crosshair"
          >
            <text
              fontSize={14}
              fontWeight={700}
              color={hovering ? C.accentHi : C.text}
            >
              {hovering ? 'Inside' : 'Move the cursor here'}
            </text>
            <text fontSize={12} color={C.textMuted}>
              {pos
                ? `window x:${pos.x} y:${pos.y}  ·  local x:${pos.localX} y:${pos.localY}`
                : 'derived from a single mousemove'}
            </text>
          </view>
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <text fontSize={14} fontWeight={700} color={C.text}>
              onBeforeInput
            </text>
            <Badge
              label={`${blocked} blocked`}
              color={C.warningHi}
              bg={C.warningDim}
            />
          </view>
          <text fontSize={12} color={C.textMuted}>
            preventDefault() in onBeforeInput stops the edit before it commits.
            This field rejects digits.
          </text>
          <input
            value={filtered}
            placeholder="Try typing letters and numbers"
            onBeforeInput={(e) => {
              if (e.data && /\d/.test(e.data)) {
                e.preventDefault();
                setBlocked((b) => b + 1);
                push('onBeforeInput');
              }
            }}
            onValueChange={setFiltered}
            px={12}
            py={10}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            focus:borderColor={C.accent}
            color={C.text}
            fontSize={14}
          />
        </view>

        <view display="flex" flexDir="col" gap={10}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            onCommit
          </text>
          <text fontSize={12} color={C.textMuted}>
            Fires on blur only if the value changed since focus (not per
            keystroke). Edit and click away, or press Tab.
          </text>
          <input
            placeholder="Type, then blur to commit"
            onCommit={(e) => {
              setCommitted(e.data ?? '');
              push('onCommit');
            }}
            px={12}
            py={10}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            focus:borderColor={C.accent}
            color={C.text}
            fontSize={14}
          />
          <view>
            <text fontSize={12} color={C.textMuted}>
              last committed:{' '}
            </text>
            <text fontSize={12} fontWeight={700} color={C.primaryHi}>
              {committed ? `"${committed}"` : 'nothing yet'}
            </text>
          </view>

          <view display="flex" flexDir="row" items="center" gap={8} mt={4}>
            <checkbox
              checked={checked}
              onCommit={() => push('onCommit')}
              onValueChange={setChecked}
            />
            <text fontSize={12} color={C.textMuted}>
              checkbox commits immediately on toggle (currently{' '}
            </text>
            <text fontSize={12} fontWeight={700} color={C.primaryHi}>
              {checked ? 'on' : 'off'}
            </text>
            <text fontSize={12} color={C.textMuted}>
              {'}'}
            </text>
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
            <view display="flex" flexDir="row" items="center" gap={8}>
              <text fontSize={14} fontWeight={700} color={C.text}>
                Event Log
              </text>
              <Badge
                label={`${seq} total`}
                color={C.accentHi}
                bg={C.accentDark}
              />
            </view>
            <button
              onClick={() => {
                setEventLog([]);
                setClicks(0);
                setDowns(0);
                setUps(0);
                setSeq(0);
                setBlocked(0);
                setCommitted('');
                setChecked(false);
              }}
              px={12}
              py={5}
              bg={C.dangerDim}
              hover:bg="#991b1b"
              rounded={8}
              cursor="pointer"
              border={1}
              borderColor={C.danger}
            >
              <text fontSize={12} fontWeight={600} color={C.dangerHi}>
                Reset
              </text>
            </button>
          </view>
          <view scroll h={220} display="flex" flexDir="col">
            {eventLog.length === 0 ? (
              <view p={20} display="flex" items="center" justify="center">
                <text fontSize={13} color={C.textMuted}>
                  Interact with the hit target above to see events here.
                </text>
              </view>
            ) : (
              eventLog.map((e, _i) => (
                <view
                  key={e.n}
                  display="flex"
                  flexDir="row"
                  items="center"
                  gap={12}
                  py={6}
                  borderBottom={1}
                  borderColor={C.border}
                >
                  <text fontSize={11} color={C.textMuted} fontWeight={700}>
                    #{String(e.n).padStart(4, '0')}
                  </text>
                  <view w={6} h={6} bg={typeColor(e.type)} rounded={4} />
                  <text
                    fontSize={12}
                    fontWeight={600}
                    color={typeColor(e.type)}
                  >
                    {e.type}
                  </text>
                  <text fontSize={11} color={C.textMuted}>
                    {new Date(e.ts).toLocaleTimeString([], {
                      hour: '2-digit',
                      minute: '2-digit',
                      second: '2-digit',
                    })}
                  </text>
                </view>
              ))
            )}
          </view>
        </view>
      </view>
    </view>
  );
}
