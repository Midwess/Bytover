//
//  Modal.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation
import SharedTypes

enum ShareSelection: String {
    case People
    case PublicUrl
    case Devices
}

extension TransferMethodSelection {
    static var allCases: [TransferMethodSelection] {
        [.device, .user, .internet]
    }

    var name: String {
        get {
            switch self {
            case .device: return "Your devices"
            case .user: return "People"
            case .internet: return "Public URL"
            }
        }
    }

    func icon(_ model: NearbyViewModel?) -> AnyView {
        switch self {
        case .device: return AnyView(ImageAsset.DeviceEmpty.image)
        case .user:
            if let peer_avatar = model?.me?.avatar {
                return AnyView(Avartar(avatar: peer_avatar))
            }
            return AnyView(ImageAsset.UserEmpty.image)
        case .internet: return AnyView(ImageAsset.GlobeEmpty.image)
        }
    }

    var body: AnyView {
        get {
            switch self {
            case .device: return AnyView(DeviceShareView())
            case .user: return AnyView(PeopleShareView())
            case .internet: return AnyView(PublicUrlShareView())
            }
        }
    }
}

struct ShareModal: View {
    @EnvironmentObject var core: Core
    @State private var nearby: NearbyViewModel?
    @State private var selection = TransferMethodSelection.internet

    let selections = TransferMethodSelection.allCases

    var body: some View {
        VStack(alignment: .leading, spacing: SpaceTheme.screen.value) {
            HStack(alignment: .center, spacing: SpaceTheme.item.value) {
                selection.icon(nearby)
                    .frame(width: 56, height: 56)
                    .fontWeight(.bold)
                    .foregroundStyle(Theme.GreenSecondary.color)
                    .padding(.all, 0)
                    .clipShape(Circle())
                    .background(Circle()
                        .foregroundStyle(Theme.PrimaryText.color.opacity(0.16))
                    )

                VStack(alignment: .leading, spacing: SpaceTheme.cohesive.halfValue) {
                    Text("Share to")
                        .modifier(Label2())
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.8))

                    HStack(spacing: SpaceTheme.cohesive.halfValue) {
                        Menu(selection.name) {
                            Button(TransferMethodSelection.device.name, action: {
                                core.selectedTransfer.value = .device
                            })
                            Button(TransferMethodSelection.user.name, action: {
                                core.selectedTransfer.value = .user
                            })
                            Button(TransferMethodSelection.internet.name, action: {
                                core.selectedTransfer.value = .internet
                            })
                        }
                        .modifier(Body1())
                        .foregroundStyle(Theme.PrimaryText.color)
                        .background(.clear)

                        ImageAsset.ArrowDown.image
                            .resizable()
                            .frame(width: 24, height: 24)
                    }
                }

                Spacer()
            }

            ZStack {
                TransferMethodSelection.device.body
                    .opacity(selection == .device ? 1 : 0)
                    .allowsHitTesting(selection == .device)
                TransferMethodSelection.user.body
                    .opacity(selection == .user ? 1 : 0)
                    .allowsHitTesting(selection == .user)
                TransferMethodSelection.internet.body
                    .opacity(selection == .internet ? 1 : 0)
                    .allowsHitTesting(selection == .internet)
            }

            Spacer()
        }
        .padding(.horizontal, SpaceTheme.screen.value)
        .padding(.top, SpaceTheme.screen.value)
        .background(
            .clear
        )
        .clipShape(RoundedRectangle(cornerRadius: 36))
        .shadow(radius: 2)
        .onReceive(self.core.selectedTransfer, perform: {value in
            selection = value
        })
        .onReceive(self.core.nearby, perform: {value in
            self.nearby = value
        })
    }
}

#Preview {
    ZStack {
        ShareModal()
            .previewLayout(.sizeThatFits)
            .frame(width: .infinity, height: 350)
            .environmentObject(CoreMock() as Core)
    }
    .background(Theme.BlackBase.color)
}
