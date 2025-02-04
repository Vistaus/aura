{-# LANGUAGE OverloadedLists #-}

module Main ( main ) where

import           Aura.Packages.Repository
import           Aura.Pkgbuild.Security
import           Aura.Settings.External
import           Aura.Types
import           Data.Versions
import           RIO
import qualified RIO.Map as M
import           RIO.Partial
-- import           Language.Bash.Pretty (prettyText)
import           Test.Tasty
import           Test.Tasty.HUnit
import           Text.Megaparsec
-- import           Text.Pretty.Simple (pPrintNoColor)

---

main :: IO ()
main = do
  conf <- readFileUtf8 "test/pacman.conf"
  bad  <- Pkgbuild <$> readFileBinary "test/bad.PKGBUILD"
  good  <- Pkgbuild <$> readFileBinary "test/good.PKGBUILD"
  defaultMain $ suite conf bad good

suite :: Text -> Pkgbuild -> Pkgbuild -> TestTree
suite conf badRaw goodRaw = testGroup "Unit Tests"
  [ testGroup "Aura.Core"
    [ testCase "parseDep - python2" $ parseDep "python2" @?= Just (Dep "python2" Anything)
    , testCase "parseDep - python2-lxml>=3.1.0" $ do
        let pn = "python2-lxml>=3.1.0"
            p = parseDep pn
        p @?= Just (Dep "python2-lxml" . AtLeast . Ideal $ SemVer 3 1 0 [] Nothing)
        fmap renderedDep p @?= Just pn
    , testCase "parseDep - foobar>1.2.3" $ do
        let pn = "foobar>1.2.3"
            p = parseDep pn
        p @?= Just (Dep "foobar" . MoreThan . Ideal $ SemVer 1 2 3 [] Nothing)
        fmap renderedDep p @?= Just pn
    , testCase "parseDep - foobar=1.2.3" $ do
        let pn = "foobar=1.2.3"
            p = parseDep pn
        p @?= Just (Dep "foobar" . MustBe . Ideal $ SemVer 1 2 3 [] Nothing)
        fmap renderedDep p @?= Just pn
    ]
  , testGroup "Aura.Types"
    [ testCase "simplepkg"
      $ simplepkg (fromJust $ packagePath "/var/cache/pacman/pkg/linux-is-cool-3.2.14-1-x86_64.pkg.tar.xz")
      @?= Just (SimplePkg "linux-is-cool" . Ideal $ SemVer 3 2 14 [[Digits 1]] Nothing)
    , testCase "simplepkg'"
      $ simplepkg' "xchat 2.8.8-19" @?= Just (SimplePkg "xchat" . Ideal $ SemVer 2 8 8 [[Digits 19]] Nothing)
    ]
  , testGroup "Aura.Packages.Repository"
    [ testCase "extractVersion" $ extractVersion firefox @?= Just (Ideal $ SemVer 60 0 2 [[Digits 1]] Nothing)
    ]
  , testGroup "Aura.Pacman"
    [ testCase "Parsing pacman.conf" $ do
        let p = parse config "pacman.conf" conf
            r = either (const Nothing) (\(Config c) -> Just c) p
        (r >>= M.lookup "HoldPkg") @?= Just ["pacman", "glibc"]
        (r >>= M.lookup "GoodLine") @?= Just ["good"]
        (r >>= M.lookup "BadLine") @?= Just ["bad"]
        (r >>= M.lookup "OkLine") @?= Just ["ok"]
        (r >>= M.lookup "ParallelDownloads") @?= Just ["5"]
    ]
  , testGroup "Aura.Pkgbuild.Security"
    [ testCase "Parsing - bad.PKGBUILD" $
        -- pPrintNoColor $ map (first prettyText) . bannedTerms <$> ppb
        -- pPrintNoColor ppb
        assertBool "Failed to parse" $ isJust bad
    , testCase "Detecting banned terms" $ (length . bannedTerms <$> bad) @?= Just 5
    , testCase "Parsing - good.PKGBUILD" $ assertBool "Failed to parse" $ isJust good
    , testCase "Shouldn't find anything" $ (length . bannedTerms <$> good) @?= Just 0
    ]
  ]
  where
    bad = parsedPB badRaw
    good = parsedPB goodRaw

firefox :: Text
firefox = "Repository      : extra\n\
\Name            : firefox\n\
\Version         : 60.0.2-1\n\
\Description     : Standalone web browser from mozilla.org"
