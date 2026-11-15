import React, { useState, useEffect } from 'react';
import { Gamepad2 } from 'lucide-react';
import { safeInvoke } from '../ipc';

interface CoverImageProps {
  assetId?: string | null;
  title?: string;
  className?: string;
  aspectRatio?: '2:3' | '1:1';
}

export const CoverImage: React.FC<CoverImageProps> = ({
  assetId,
  title = '',
  className = '',
}) => {
  const [thumbUrl, setThumbUrl] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (assetId) {
      safeInvoke<{ data_url: string } | null>(
        'get_asset_thumbnail',
        {
          assetId,
          maxDimension: 450,
        },
        true
      )
        .then((res) => {
          if (active && res?.data_url) {
            setThumbUrl(res.data_url);
          }
        })
        .catch(() => {
          if (active) setThumbUrl(null);
        });
    } else {
      setThumbUrl(null);
    }
    return () => {
      active = false;
    };
  }, [assetId]);

  if (thumbUrl) {
    return (
      <div
        className={`relative overflow-hidden bg-neutral-950 flex items-center justify-center select-none ${className}`}
      >
        {/* Ambient blurred backdrop for adaptive framing (e.g. 1:1 square icon in 2:3 card) */}
        <img
          src={thumbUrl}
          alt=""
          className="absolute inset-0 w-full h-full object-cover blur-md opacity-35 scale-110 pointer-events-none"
        />
        {/* Main sharp image */}
        <img
          src={thumbUrl}
          alt={title}
          className="relative max-h-full max-w-full object-contain shadow-md rounded"
          loading="lazy"
        />
      </div>
    );
  }

  // Fallback card when no cover image is linked
  return (
    <div
      className={`flex flex-col items-center justify-center bg-gradient-to-br from-neutral-900 to-neutral-950 border border-neutral-800 text-neutral-500 p-4 select-none ${className}`}
    >
      <Gamepad2 className="w-9 h-9 mb-1.5 opacity-30 text-emerald-400" />
      <span className="text-[11px] font-medium text-center line-clamp-2 text-neutral-400 px-1">
        {title}
      </span>
    </div>
  );
};
