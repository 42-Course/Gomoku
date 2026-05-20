import { useMemo, useState } from "react";
import type {
  Board as BoardGrid,
  Coord,
  Player,
} from "@/api/types";
import { BOARD_SIZE } from "@/api/fixtures";

interface BoardProps {
  board: BoardGrid;
  lastMove?: Coord | null;
  showLastMove?: boolean;
  /** A–T / 1–19 labels around the board edges. */
  showCoordinates?: boolean;
  /** Vertical + horizontal guide lines following the cursor. */
  showCrosshair?: boolean;
  /** Translucent ghost stone at the hovered cell. */
  showHoverGhost?: boolean;
  /** Whose stone the ghost should show (the side to move). */
  currentPlayer?: Player;
  highlightCoord?: Coord | null;
  onCellClick?: (c: Coord) => void;
  /** Fired with the cell under the cursor, or null on leave. */
  onHoverCell?: (c: Coord | null) => void;
}

const PAD = 36; // outer padding inside SVG viewBox (room for coord labels)
const LINE = 32; // px spacing between grid lines
const SIZE = PAD * 2 + LINE * (BOARD_SIZE - 1);

const STAR_POINTS: Coord[] = [
  { x: 3, y: 3 }, { x: 9, y: 3 }, { x: 15, y: 3 },
  { x: 3, y: 9 }, { x: 9, y: 9 }, { x: 15, y: 9 },
  { x: 3, y: 15 }, { x: 9, y: 15 }, { x: 15, y: 15 },
];

/** A..T skipping "I" — gomoku/renju/gomocup convention. */
const COL_LETTERS = "ABCDEFGHJKLMNOPQRST";

function cellToPx({ x, y }: Coord) {
  return { cx: PAD + x * LINE, cy: PAD + y * LINE };
}

export function Board({
  board,
  lastMove,
  showLastMove = true,
  showCoordinates = true,
  showCrosshair = true,
  showHoverGhost = true,
  currentPlayer,
  highlightCoord,
  onCellClick,
  onHoverCell,
}: BoardProps) {
  const [hover, setHoverState] = useState<Coord | null>(null);
  const setHover = (c: Coord | null) => {
    setHoverState(c);
    onHoverCell?.(c);
  };

  const stones = useMemo(() => {
    const out: { coord: Coord; player: "black" | "white" }[] = [];
    for (let y = 0; y < BOARD_SIZE; y++) {
      for (let x = 0; x < BOARD_SIZE; x++) {
        const p = board[y][x];
        if (p) out.push({ coord: { x, y }, player: p });
      }
    }
    return out;
  }, [board]);

  const occupied = (c: Coord) => board[c.y]?.[c.x] != null;

  return (
    <div className="relative mx-auto aspect-square w-full max-w-[720px]">
      <svg viewBox={`0 0 ${SIZE} ${SIZE}`} className="h-full w-full rounded-md shadow-lg">
        <defs>
          <radialGradient id="wood" cx="50%" cy="40%" r="75%">
            <stop offset="0%" stopColor="#e6b980" />
            <stop offset="100%" stopColor="#c77b37" />
          </radialGradient>
          <radialGradient id="blackStone" cx="35%" cy="30%" r="70%">
            <stop offset="0%" stopColor="#4a4a4a" />
            <stop offset="50%" stopColor="#1f1f1f" />
            <stop offset="100%" stopColor="#0a0a0a" />
          </radialGradient>
          <radialGradient id="whiteStone" cx="35%" cy="30%" r="70%">
            <stop offset="0%" stopColor="#ffffff" />
            <stop offset="70%" stopColor="#ece5d7" />
            <stop offset="100%" stopColor="#b9af99" />
          </radialGradient>
        </defs>
        <rect width={SIZE} height={SIZE} fill="url(#wood)" rx={6} ry={6} />

        {/* Coordinate labels: letters along top & bottom, numbers on left & right. */}
        {showCoordinates && (
          <g
            fill="#4a2c16"
            fontFamily="ui-monospace, monospace"
            fontSize={11}
            opacity={0.75}
            pointerEvents="none"
          >
            {Array.from({ length: BOARD_SIZE }).map((_, i) => {
              const px = PAD + LINE * i;
              return (
                <g key={`col-${i}`}>
                  <text x={px} y={PAD - 14} textAnchor="middle">
                    {COL_LETTERS[i]}
                  </text>
                  <text x={px} y={SIZE - PAD + 22} textAnchor="middle">
                    {COL_LETTERS[i]}
                  </text>
                </g>
              );
            })}
            {Array.from({ length: BOARD_SIZE }).map((_, i) => {
              const py = PAD + LINE * i;
              const num = (i + 1).toString();
              return (
                <g key={`row-${i}`}>
                  <text x={PAD - 14} y={py + 4} textAnchor="middle">
                    {num}
                  </text>
                  <text x={SIZE - PAD + 14} y={py + 4} textAnchor="middle">
                    {num}
                  </text>
                </g>
              );
            })}
          </g>
        )}

        {/* Grid lines */}
        <g stroke="#4a2c16" strokeWidth={1}>
          {Array.from({ length: BOARD_SIZE }).map((_, i) => (
            <line
              key={`h${i}`}
              x1={PAD}
              x2={PAD + LINE * (BOARD_SIZE - 1)}
              y1={PAD + LINE * i}
              y2={PAD + LINE * i}
            />
          ))}
          {Array.from({ length: BOARD_SIZE }).map((_, i) => (
            <line
              key={`v${i}`}
              x1={PAD + LINE * i}
              x2={PAD + LINE * i}
              y1={PAD}
              y2={PAD + LINE * (BOARD_SIZE - 1)}
            />
          ))}
        </g>

        {/* Star points */}
        <g fill="#4a2c16">
          {STAR_POINTS.map((s) => {
            const { cx, cy } = cellToPx(s);
            return <circle key={`${s.x}-${s.y}`} cx={cx} cy={cy} r={3} />;
          })}
        </g>

        {/*
          Crosshair from the hovered cell. Solid white-ish line at high
          opacity reads on the orange wood gradient; the previous
          orange-dashed version vanished into the background.
        */}
        {showCrosshair && hover && (
          <g stroke="#fff8e7" strokeWidth={2} strokeOpacity={0.7} pointerEvents="none">
            <line
              x1={PAD}
              x2={PAD + LINE * (BOARD_SIZE - 1)}
              y1={PAD + hover.y * LINE}
              y2={PAD + hover.y * LINE}
            />
            <line
              x1={PAD + hover.x * LINE}
              x2={PAD + hover.x * LINE}
              y1={PAD}
              y2={PAD + LINE * (BOARD_SIZE - 1)}
            />
          </g>
        )}

        {/* Stones */}
        <g>
          {stones.map(({ coord, player }) => {
            const { cx, cy } = cellToPx(coord);
            return (
              <g key={`s-${coord.x}-${coord.y}`}>
                <circle cx={cx + 1} cy={cy + 2} r={LINE * 0.42} fill="rgba(0,0,0,0.35)" />
                <circle
                  cx={cx}
                  cy={cy}
                  r={LINE * 0.42}
                  fill={player === "black" ? "url(#blackStone)" : "url(#whiteStone)"}
                />
              </g>
            );
          })}
        </g>

        {/* Hover ghost — translucent stone preview at the cursor. */}
        {showHoverGhost && hover && currentPlayer && !occupied(hover) && (() => {
          const { cx, cy } = cellToPx(hover);
          return (
            <circle
              cx={cx}
              cy={cy}
              r={LINE * 0.42}
              fill={currentPlayer === "black" ? "url(#blackStone)" : "url(#whiteStone)"}
              opacity={0.45}
              pointerEvents="none"
            />
          );
        })()}

        {/* Last move marker */}
        {showLastMove && lastMove && (() => {
          const { cx, cy } = cellToPx(lastMove);
          return (
            <circle
              cx={cx}
              cy={cy}
              r={5}
              fill="none"
              stroke="#d85000"
              strokeWidth={2.5}
            />
          );
        })()}

        {/* Highlight from external suggestion. */}
        {highlightCoord && (() => {
          const { cx, cy } = cellToPx(highlightCoord);
          return (
            <circle
              cx={cx}
              cy={cy}
              r={LINE * 0.5}
              fill="none"
              stroke="#629924"
              strokeWidth={3}
              strokeDasharray="4 3"
            />
          );
        })()}

        {/* Click + hover targets. The mouseleave on the wrapping <g> clears hover. */}
        <g onMouseLeave={() => setHover(null)}>
          {Array.from({ length: BOARD_SIZE }).map((_, y) =>
            Array.from({ length: BOARD_SIZE }).map((_, x) => {
              const { cx, cy } = cellToPx({ x, y });
              const c: Coord = { x, y };
              return (
                <rect
                  key={`t-${x}-${y}`}
                  x={cx - LINE / 2}
                  y={cy - LINE / 2}
                  width={LINE}
                  height={LINE}
                  fill="transparent"
                  className={onCellClick ? "cursor-pointer" : ""}
                  onMouseEnter={() => setHover(c)}
                  onClick={() => onCellClick?.(c)}
                />
              );
            }),
          )}
        </g>
      </svg>
    </div>
  );
}
