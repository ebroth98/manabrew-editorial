import { forwardRef } from "react";

export const ScryfallImg = forwardRef<HTMLImageElement, React.ImgHTMLAttributes<HTMLImageElement>>(
  function ScryfallImg(props, ref) {
    return <img ref={ref} {...props} crossOrigin="anonymous" />;
  },
);
