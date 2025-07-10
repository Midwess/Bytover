import {LiquidButton} from '@/components/animate-ui/buttons/liquid'
import {GitHubStarsButton} from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";

export default function Header() {
    return (
        <div className="relative flex justify-between items-center w-full py-10 container">
            <div className="flex flex-row gap-2 items-center">
                <Image
                    width={40}
                    height={40}
                    src="logo.svg"
                    alt="Logo"
                />
                <p className="text-2xl font-poppins font-bold">Bit bridge</p>
            </div>

            <div className="absolute left-1/2 transform -translate-x-1/2">
                <div className="flex flex-row gap-5 rounded-full border border-primaryText/30 px-8 py-2">
                    {["About", "Pricing", "How it works"].map((item) => (
                        <a
                            key={item}
                            href="#"
                            className="nav-link font-poppins text-primaryText/80"
                        >
                            {item}
                        </a>
                    ))}
                </div>

                <style>{`
                    .nav-link {
                      position: relative;
                      text-decoration: none;
                    }
                    .nav-link::before {
                      content: '';
                      position: absolute;
                      bottom: 0;
                      left: 0;
                      width: 0;
                      height: 2px;
                      background-color: rgba(255, 253, 246, 0.8); /* primaryText/80 */
                      transition: width 300ms ease;
                    }
                    .nav-link:hover::before {
                      width: 100%;
                    }
                `}
                </style>

            </div>
            <div className="flex flex-row gap-2 font-poppins font-bold text-primaryText">
                <LiquidButton variant={"outline"}>Sign up</LiquidButton>
                <GitHubStarsButton className={"bg-white/90"} username="Dev-log" repo="animate-ui"/>
            </div>
        </div>
    )
}
