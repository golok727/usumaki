import { useRef, useState } from 'react';
import { UzInputElement } from 'uzumaki';
import { C } from '../theme';
import { Divider, Badge } from '../components';

export function InputsPage() {
  const [
    { username, email, password, confirm, bio, search, submitted },
    setFormState,
  ] = useState({
    username: '',
    email: '',
    password: '',
    confirm: '',
    bio: '',
    search: '',
    submitted: false,
  });

  const reset = () =>
    setFormState({
      username: '',
      email: '',
      password: '',
      confirm: '',
      bio: '',
      search: '',
      submitted: false,
    });

  const otpRefs = [
    useRef<UzInputElement | null>(null),
    useRef<UzInputElement | null>(null),
    useRef<UzInputElement | null>(null),
    useRef<UzInputElement | null>(null),
    useRef<UzInputElement | null>(null),
    useRef<UzInputElement | null>(null),
  ];
  const otpLength = otpRefs.length;
  const [otp, setOtp] = useState<string[]>(() =>
    Array.from({ length: otpLength }, () => ''),
  );
  const otpComplete = otp.every((d) => d.length === 1);

  const [noRounding, setNoRounding] = useState(false);
  const [rounded, setRounded] = useState(true);
  const [circle, setCircle] = useState(false);

  const inputPadding = 8;

  const pwMatch = password === confirm && confirm.length > 0;
  const pwMismatch = confirm.length > 0 && password !== confirm;

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
        px={24}
        py={16}
        borderBottom={1}
        borderColor={C.border}
      >
        <text fontSize={20} fontWeight={800} color={C.text}>
          Input Lab
        </text>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <view display="flex" flexDir="col" gap={8}>
          <text fontSize={13} fontWeight={600} color={C.textSub}>
            Search Field
          </text>
          <input
            value={search}
            onValueChange={(v) =>
              setFormState((prev) => ({ ...prev, search: v }))
            }
            placeholder="Search anything... (try IME input)"
            fontSize={15}
            color={C.text}
            bg={C.surface2}
            p={inputPadding}
            rounded={8}
            border={1}
            borderColor={search.length > 0 ? C.accent : C.border}
            w="full"
          />
          <view display="flex" flexDir="row" items="center" gap={8}>
            <Badge
              label={`${search.length} chars`}
              color={search.length > 0 ? C.accentHi : C.textMuted}
              bg={search.length > 0 ? C.accentDark : C.surface3}
            />
            {search.length > 20 && (
              <Badge
                label="LONG QUERY"
                color={C.warningHi}
                bg={C.warningDark}
              />
            )}
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="row" gap={16}>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Username
            </text>
            <input
              value={username}
              onValueChange={(v) =>
                setFormState((prev) => ({ ...prev, username: v }))
              }
              placeholder="johndoe"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Email
            </text>
            <input
              value={email}
              onValueChange={(v) =>
                setFormState((prev) => ({ ...prev, email: v }))
              }
              placeholder="john@example.com"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
        </view>

        <view display="flex" flexDir="row" gap={16}>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Password (secure)
            </text>
            <input
              secure
              value={password}
              onValueChange={(v) =>
                setFormState((prev) => ({ ...prev, password: v }))
              }
              placeholder="Enter password"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={C.border}
              w="full"
            />
          </view>
          <view display="flex" flexDir="col" gap={8} flex={1}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Confirm Password
            </text>
            <input
              secure
              value={confirm}
              onValueChange={(v) =>
                setFormState((prev) => ({ ...prev, confirm: v }))
              }
              placeholder="Repeat password"
              fontSize={14}
              color={C.text}
              bg={C.surface2}
              p={inputPadding}
              rounded={8}
              border={1}
              borderColor={
                pwMismatch ? C.danger : (pwMatch ? C.success : C.border)
              }
              w="full"
            />
            {pwMismatch && (
              <text fontSize={11} color={C.danger}>
                Passwords don't match
              </text>
            )}
            {pwMatch && (
              <>
                <view gap={2}>
                  <text fontSize={11} color={C.success}>
                    Passwords match
                  </text>
                </view>
              </>
            )}
          </view>
        </view>

        <view display="flex" flexDir="col" gap={8}>
          <view display="flex" flexDir="row" items="center" justify="between">
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Bio (multiline)
            </text>
            <text
              fontSize={11}
              color={bio.length > 200 ? C.danger : C.textMuted}
            >
              {bio.length}/300
            </text>
          </view>
          <input
            multiline
            value={bio}
            onValueChange={(v) => setFormState((prev) => ({ ...prev, bio: v }))}
            placeholder="Tell us about yourself... (multiline input, try pasting long text)"
            fontSize={16}
            color={C.text}
            bg={C.surface2}
            p={12}
            rounded={8}
            border={1}
            borderColor={bio.length > 200 ? C.warning : C.border}
            w="full"
            h={110}
          />
        </view>

        <view display="flex" flexDir="row" gap={8}>
          <button
            display="flex"
            flexDir="col"
            justify="center"
            onClick={() =>
              setFormState((prev) => ({ ...prev, submitted: true }))
            }
            px={24}
            h={36}
            bg={C.accent}
            hover:bg={C.warning}
            active:bg={C.warningDark}
            rounded={8}
            cursor="pointer"
          >
            <text fontSize={14} fontWeight={700} color="#fff">
              Submit Form
            </text>
          </button>
          <button
            display="flex"
            flexDir="col"
            justify="center"
            onClick={() => {
              reset();
            }}
            px={24}
            h={36}
            bg={C.surface3}
            hover:bg={C.surface4}
            active:bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
            cursor="pointer"
          >
            <text fontSize={14} color={C.textMuted}>
              Reset
            </text>
          </button>
        </view>

        {submitted && (
          <view
            p={16}
            bg={C.successDim}
            rounded={8}
            border={1}
            borderColor={C.success}
            display="flex"
            flexDir="col"
            gap={4}
          >
            <view gap={2}>
              <text fontSize={14} fontWeight={700} color={C.successHi}>
                Form submitted
              </text>
            </view>

            <text fontSize={12} color={C.success}>
              user={username || '(empty)'} · email={email || '(empty)'} · bio=
              {bio.length} chars
            </text>
          </view>
        )}

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Enter Otp
          </text>
          <view
            display="flex"
            flexDir="col"
            gap={12}
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <view display="flex" flexDir="row" gap={8}>
              {otp.map((digit, i) => (
                <input
                  key={i}
                  ref={otpRefs[i]}
                  value={digit}
                  maxLength={1}
                  placeholder="-"
                  onValueChange={(v) => {
                    const next = v.slice(-1);
                    setOtp((prev) => {
                      const out = [...prev];
                      out[i] = next;
                      return out;
                    });
                    if (next && i < otpLength - 1) {
                      otpRefs[i + 1]?.current?.focus();
                    }
                  }}
                  onKeyDown={(ev) => {
                    if (ev.key === 'Backspace' && !otp[i] && i > 0) {
                      ev.preventDefault();
                      setOtp((prev) => {
                        const out = [...prev];
                        out[i - 1] = '';
                        return out;
                      });
                      otpRefs[i - 1]?.current?.focus();
                    } else if (ev.key === 'ArrowLeft' && i > 0) {
                      ev.preventDefault();
                      otpRefs[i - 1]?.current?.focus();
                    } else if (ev.key === 'ArrowRight' && i < otpLength - 1) {
                      ev.preventDefault();
                      otpRefs[i + 1]?.current?.focus();
                    } else if (
                      ev.key.length === 1 &&
                      !ev.ctrlKey &&
                      !ev.metaKey &&
                      !ev.altKey &&
                      otp[i]
                    ) {
                      ev.preventDefault();
                      setOtp((prev) => {
                        const out = [...prev];
                        out[i] = ev.key;
                        return out;
                      });
                      if (i < otpLength - 1) {
                        otpRefs[i + 1]?.current?.focus();
                      }
                    }
                  }}
                  fontSize={20}
                  fontWeight={700}
                  color={C.text}
                  bg={C.surface3}
                  pr={0}
                  py={0}
                  textAlign="center"
                  w={44}
                  h={44}
                  rounded={8}
                  border={1}
                  borderColor={digit ? C.accent : C.border}
                  focus:borderColor={C.accentHi}
                />
              ))}
            </view>
            <view display="flex" flexDir="row" items="center" gap={10}>
              <button
                onClick={() => {
                  setOtp(Array.from({ length: otpLength }, () => ''));
                  otpRefs[0]?.current?.focus();
                }}
                display="flex"
                flexDir="col"
                justify="center"
                px={14}
                h={32}
                bg={C.surface3}
                hover:bg={C.surface4}
                rounded={6}
                border={1}
                borderColor={C.border}
                cursor="pointer"
              >
                <text fontSize={12} color={C.textMuted}>
                  Clear & focus
                </text>
              </button>
              {otpComplete && (
                <view display="flex" flexDir="row" items="center" gap={6}>
                  <text fontSize={12} color={C.success}>
                    Code: {otp.join('')}
                  </text>
                </view>
              )}
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={12}>
          <text fontSize={14} fontWeight={700} color={C.text}>
            Checkboxes
          </text>
          <view
            display="flex"
            flexDir="col"
            p={16}
            gap={14}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.border}
          >
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={noRounding}
                onValueChange={setNoRounding}
                bg={C.accent}
                borderColor={noRounding ? C.accent : C.border}
                color="#ffffff"
                w={20}
                h={20}
                hover:opacity={0.9}
              />
              <text fontSize={14} color={C.text}>
                Square checkbox{noRounding ? ' [selected]' : ''}
              </text>
            </view>
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={rounded}
                onValueChange={setRounded}
                bg={C.success}
                borderColor={rounded ? C.success : C.border}
                color="#08110a"
                rounded={4}
                w={20}
                h={20}
              />
              <text fontSize={14} color={C.text}>
                Rounded checkbox{rounded ? ' [selected]' : ''}
              </text>
            </view>
            <view display="flex" items="center" gap={12}>
              <checkbox
                checked={circle}
                onValueChange={setCircle}
                bg={C.warning}
                borderColor={circle ? C.warning : C.border}
                color="#1b1104"
                rounded={10}
                w={20}
                h={20}
              />
              <text fontSize={14} color={C.text}>
                Circular checkbox{circle ? ' [selected]' : ''}
              </text>
            </view>
          </view>
        </view>

        <Divider />

        <view display="flex" flexDir="col" gap={8}>
          <view display="flex" flexDir="row" items="center" gap={8}>
            <text fontSize={13} fontWeight={600} color={C.textSub}>
              Selectable Text Block
            </text>
          </view>
          <view
            selectable
            w="full"
            p={16}
            bg={C.surface2}
            rounded={8}
            border={1}
            borderColor={C.borderHi}
          >
            <text fontSize={13} color={C.textDim} w="100%">
              The quick brown fox jumps over the lazy dog. Pack my box with five
              dozen liquor jugs. How valiantly the strong and quick brown fox
              leaps over the sleeping lazy hound dog! Try selecting this text
              with your mouse.
            </text>
          </view>
        </view>
      </view>
    </view>
  );
}
