import React from "react";

export function Button(props: { label: string }) {
  return <button>{props.label}</button>;
}

export const Label = (props: { text: string }) => {
  return <span>{props.text}</span>;
};
