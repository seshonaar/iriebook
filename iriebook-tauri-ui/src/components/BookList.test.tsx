import { render, screen } from "@testing-library/react";
import * as React from "react";
import { describe, expect, it } from "vitest";
import { AppProvider, useAppContext } from "../contexts/AppContext";
import { setBooks, setFolder } from "../contexts/actions";
import { BookList } from "./BookList";

function TestHarness() {
  const { dispatch } = useAppContext();

  React.useEffect(() => {
    dispatch(setFolder("/workspace/books"));
    dispatch(setBooks([]));
  }, [dispatch]);

  return <BookList />;
}

describe("BookList", () => {
  it("shows the add book action when the selected workspace is empty", async () => {
    render(
      <AppProvider>
        <TestHarness />
      </AppProvider>
    );

    expect(await screen.findByText("books.list.noBooks")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /books\.list\.addBookFrom/i })
    ).toBeInTheDocument();
  });
});
