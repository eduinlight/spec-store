import { alternates, Landing, metaFor } from "~/landing";

export function meta() {
  return metaFor("en");
}

export function links() {
  return alternates();
}

export default function Home() {
  return <Landing language="en" />;
}
