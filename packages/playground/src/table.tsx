import type { ReactNode } from 'react';
import { C } from './theme';

export interface TableColumn<T> {
  key: keyof T | string;
  header: string;
  width?: number | string;
  flex?: number;
  align?: 'start' | 'center' | 'end';
  render?: (row: T, index: number) => ReactNode;
}

interface TableProps<T> {
  columns: TableColumn<T>[];
  data: T[];
  keyField: keyof T;
  rowHeight?: number;
  headerBg?: string;
  rowBg?: string;
  rowHoverBg?: string;
}

export function Table<T>({
  columns,
  data,
  keyField,
  rowHeight = 44,
  headerBg = C.surface3,
  rowBg = C.surface2,
  rowHoverBg = C.surface3,
}: TableProps<T>) {
  return (
    <view
      display="flex"
      flexDir="col"
      border={1}
      borderColor={C.border}
      rounded={8}
    >
      <view
        display="flex"
        flexDir="row"
        bg={headerBg}
        borderBottom={1}
        borderColor={C.border}
      >
        {columns.map((col, _i) => (
          <view
            key={String(col.key)}
            flex={col.flex}
            w={col.flex ? undefined : (col.width ?? 120)}
            h={rowHeight}
            px={12}
            display="flex"
            items="center"
            justify={col.align}
          >
            <text fontSize={11} fontWeight={700} color={C.textMuted}>
              {col.header}
            </text>
          </view>
        ))}
      </view>
      <view display="flex" flexDir="col">
        {data.map((row, rowIndex) => (
          <view
            key={String(row[keyField])}
            display="flex"
            flexDir="row"
            bg={rowIndex % 2 === 0 ? rowBg : C.surface}
            borderBottom={1}
            borderColor={C.border}
            hover:bg={rowHoverBg}
          >
            {columns.map((col, _colIndex) => (
              <view
                key={String(col.key)}
                flex={col.flex}
                w={col.flex ? undefined : (col.width ?? 120)}
                h={rowHeight}
                px={12}
                display="flex"
                items="center"
                justify={col.align}
                minW={0}
                textWrap="wrap"
              >
                {col.render ? (
                  col.render(row, rowIndex)
                ) : (
                  <text fontSize={12} color={C.text}>
                    {String(row[col.key as keyof T] ?? '')}
                  </text>
                )}
              </view>
            ))}
          </view>
        ))}
      </view>
    </view>
  );
}
