import { useState } from 'react';
import { C } from '../theme';
import { Badge } from '../components';

const localSvgUrl = Uz.path.resource('assets/logo.svg');
const remoteImageUrl = 'https://picsum.photos/id/237/800/400';

type Status = 'idle' | 'loading' | 'loaded' | 'error';

function statusBadge(status: Status) {
  switch (status) {
    case 'loading': {
      return <Badge label="LOADING" color={C.accentHi} bg={C.accentDark} />;
    }
    case 'loaded': {
      return <Badge label="LOADED" color={C.successHi} bg={C.successDim} />;
    }
    case 'error': {
      return <Badge label="ERROR" color={C.dangerHi} bg={C.dangerDim} />;
    }
    default: {
      return <Badge label="IDLE" color={C.textMuted} bg={C.surface3} />;
    }
  }
}

function ImageCard({
  title,
  src,
  width,
  height,
}: {
  title: string;
  src: string;
  width: number;
  height: number;
}) {
  const [status, setStatus] = useState<Status>('idle');
  const [error, setError] = useState<string | null>(null);

  return (
    <view
      display="flex"
      flexDir="col"
      gap={12}
      p={18}
      bg={C.surface2}
      rounded={12}
      border={1}
      borderColor={C.border}
    >
      <view
        display="flex"
        flexDir="row"
        items="center"
        justify="between"
        fontSize={15}
        fontWeight={800}
        color={C.text}
      >
        {title}
        {statusBadge(status)}
      </view>
      <image
        src={src}
        w={width}
        h={height}
        rounded={10}
        border={1}
        borderColor={C.border}
        onLoadStart={() => {
          setStatus('loading');
          setError(null);
        }}
        onLoad={() => setStatus('loaded')}
        onError={(ev) => {
          setStatus('error');
          setError(ev.message);
        }}
      />
      <text fontSize={12} color={C.textMuted}>
        Source: {src}
      </text>
      {error && (
        <text fontSize={12} color={C.dangerHi}>
          {error}
        </text>
      )}
    </view>
  );
}

export function ImagesPage() {
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
        gap={8}
        borderBottom={1}
        borderColor={C.border}
      >
        <view fontSize={20} fontWeight={800} color={C.text}>
          Images
        </view>
        <view fontSize={12} color={C.textMuted}>
          Raster, SVG, and lifecycle event handling
        </view>
      </view>

      <view display="flex" flexDir="col" gap={20} p={24}>
        <ImageCard
          title="Local SVG"
          src={localSvgUrl}
          width={260}
          height={260}
        />

        <ImageCard
          title="Remote Raster"
          src={remoteImageUrl}
          width={420}
          height={210}
        />

        <ImageCard
          title="Broken Source (Fallback)"
          src="https://example.invalid/missing.png"
          width={260}
          height={160}
        />
      </view>
    </view>
  );
}
